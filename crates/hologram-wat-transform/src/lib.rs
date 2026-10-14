// SPDX-License-Identifier: LicenseRef-Sovereign-Leviathan-SL-AGPL3-001
//! XML DAG DSL to pure WAT transformer.
//!
//! This crate is the Rust implementation of the XML -> WAT pipeline.
//! It replaces the XSLT + Python path with a native Rust transformer
//! that parses XML DAG DSL and emits pure WebAssembly Text.
//!
//! The `wat` crate validates the generated WAT by parsing it to WASM binary.
//! If the WAT is invalid, the transform fails. No invalid WAT escapes.

use anyhow::{Context, Result};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::collections::HashMap;

/// A DAG node (one instruction).
#[derive(Debug, Clone)]
pub enum DagNode {
    Const { ty: String, value: String },
    LocalGet { name: String },
    LocalSet { name: String, body: Vec<DagNode> },
    Call { func: String, args: Vec<DagNode> },
    Add { ty: String, lhs: Box<DagNode>, rhs: Box<DagNode> },
    Sub { ty: String, lhs: Box<DagNode>, rhs: Box<DagNode> },
    Mul { ty: String, lhs: Box<DagNode>, rhs: Box<DagNode> },
    Div { ty: String, lhs: Box<DagNode>, rhs: Box<DagNode> },
    Load { ty: String, offset: u32, align: u32, addr: Box<DagNode> },
    Store { ty: String, offset: u32, align: u32, addr: Box<DagNode>, value: Box<DagNode> },
    Block { label: Option<String>, body: Vec<DagNode> },
    Loop { label: Option<String>, body: Vec<DagNode> },
    If { cond: Vec<DagNode>, then: Vec<DagNode>, els: Option<Vec<DagNode>> },
    Br { label: String },
    Return { value: Option<Box<DagNode>> },
    Drop { value: Box<DagNode> },
    // Bitwise ops (needed for real SHA-256, BLAKE3)
    And { ty: String, lhs: Box<DagNode>, rhs: Box<DagNode> },
    Or { ty: String, lhs: Box<DagNode>, rhs: Box<DagNode> },
    Xor { ty: String, lhs: Box<DagNode>, rhs: Box<DagNode> },
    Shl { ty: String, lhs: Box<DagNode>, rhs: Box<DagNode> },
    ShrU { ty: String, lhs: Box<DagNode>, rhs: Box<DagNode> },
    ShrS { ty: String, lhs: Box<DagNode>, rhs: Box<DagNode> },
    Rotr { ty: String, lhs: Box<DagNode>, rhs: Box<DagNode> },
    Rotl { ty: String, lhs: Box<DagNode>, rhs: Box<DagNode> },
    Eqz { ty: String, operand: Box<DagNode> },
    Eq { ty: String, lhs: Box<DagNode>, rhs: Box<DagNode> },
    Ne { ty: String, lhs: Box<DagNode>, rhs: Box<DagNode> },
    LtS { ty: String, lhs: Box<DagNode>, rhs: Box<DagNode> },
    LtU { ty: String, lhs: Box<DagNode>, rhs: Box<DagNode> },
    GtS { ty: String, lhs: Box<DagNode>, rhs: Box<DagNode> },
    GtU { ty: String, lhs: Box<DagNode>, rhs: Box<DagNode> },
    LeS { ty: String, lhs: Box<DagNode>, rhs: Box<DagNode> },
    LeU { ty: String, lhs: Box<DagNode>, rhs: Box<DagNode> },
    GeS { ty: String, lhs: Box<DagNode>, rhs: Box<DagNode> },
    GeU { ty: String, lhs: Box<DagNode>, rhs: Box<DagNode> },
    // Float ops
    F32Abs { operand: Box<DagNode> },
    F32Neg { operand: Box<DagNode> },
    F32Sqrt { operand: Box<DagNode> },
    F32Ceil { operand: Box<DagNode> },
    F32Floor { operand: Box<DagNode> },
    F32Trunc { operand: Box<DagNode> },
    F32Nearest { operand: Box<DagNode> },
    F32Min { lhs: Box<DagNode>, rhs: Box<DagNode> },
    F32Max { lhs: Box<DagNode>, rhs: Box<DagNode> },
    F32Copysign { lhs: Box<DagNode>, rhs: Box<DagNode> },
    // Conversions
    I32WrapI64 { operand: Box<DagNode> },
    I32TruncF32S { operand: Box<DagNode> },
    I32TruncF32U { operand: Box<DagNode> },
    I32TruncF64S { operand: Box<DagNode> },
    I32TruncF64U { operand: Box<DagNode> },
    I64ExtendI32S { operand: Box<DagNode> },
    I64ExtendI32U { operand: Box<DagNode> },
    F32ConvertI32S { operand: Box<DagNode> },
    F32ConvertI32U { operand: Box<DagNode> },
    F32ConvertI64S { operand: Box<DagNode> },
    F32ConvertI64U { operand: Box<DagNode> },
    F32DemoteF64 { operand: Box<DagNode> },
    F64ConvertI32S { operand: Box<DagNode> },
    F64ConvertI32U { operand: Box<DagNode> },
    F64ConvertI64S { operand: Box<DagNode> },
    F64ConvertI64U { operand: Box<DagNode> },
    F64PromoteF32 { operand: Box<DagNode> },
    I32ReinterpretF32 { operand: Box<DagNode> },
    I64ReinterpretF64 { operand: Box<DagNode> },
    F32ReinterpretI32 { operand: Box<DagNode> },
    F64ReinterpretI64 { operand: Box<DagNode> },
    // Memory
    MemorySize,
    MemoryGrow { operand: Box<DagNode> },
    // Control
    Unreachable,
    Nop,
    // Select
    Select { cond: Box<DagNode>, val1: Box<DagNode>, val2: Box<DagNode> },
}

/// A function definition.
#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub export: bool,
    pub params: Vec<(String, String)>,
    pub results: Vec<String>,
    pub locals: Vec<(String, String)>,
    pub body: Vec<DagNode>,
}

/// A module definition.
#[derive(Debug, Clone)]
pub struct Module {
    pub name: String,
    pub memory_pages: Option<u32>,
    pub memory_max: Option<u32>,
    pub functions: Vec<Function>,
}

/// Parse XML DAG DSL into a Module.
pub fn parse_xml(xml: &str) -> Result<Module> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut module: Option<Module> = None;
    let mut current_func: Option<Function> = None;
    let mut node_stack: Vec<Vec<DagNode>> = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let mut attrs = HashMap::new();
                for attr in e.attributes().flatten() {
                    let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                    let val = attr.unescape_value().unwrap_or_default().to_string();
                    attrs.insert(key, val);
                }
                let is_empty = matches!(reader.read_event_into(&mut buf), Ok(Event::Empty(_)));

                match name.as_str() {
                    "dag:module" | "module" => {
                        module = Some(Module {
                            name: attrs.get("name").cloned().unwrap_or_default(),
                            memory_pages: None,
                            memory_max: None,
                            functions: Vec::new(),
                        });
                    }
                    "dag:memory" | "memory" => {
                        if let Some(m) = module.as_mut() {
                            m.memory_pages = attrs.get("pages").and_then(|s| s.parse().ok());
                            m.memory_max = attrs.get("max").and_then(|s| s.parse().ok());
                        }
                    }
                    "dag:function" | "function" => {
                        current_func = Some(Function {
                            name: attrs.get("name").cloned().unwrap_or_default(),
                            export: attrs.get("export").map(|s| s == "true").unwrap_or(false),
                            params: Vec::new(),
                            results: Vec::new(),
                            locals: Vec::new(),
                            body: Vec::new(),
                        });
                        node_stack.push(Vec::new());
                    }
                    "dag:param" | "param" => {
                        if let Some(f) = current_func.as_mut() {
                            f.params.push((
                                attrs.get("name").cloned().unwrap_or_default(),
                                attrs.get("type").cloned().unwrap_or_default(),
                            ));
                        }
                    }
                    "dag:result" | "result" => {
                        if let Some(f) = current_func.as_mut() {
                            f.results.push(attrs.get("type").cloned().unwrap_or_default());
                        }
                    }
                    "dag:local" | "local" => {
                        if let Some(f) = current_func.as_mut() {
                            f.locals.push((
                                attrs.get("name").cloned().unwrap_or_default(),
                                attrs.get("type").cloned().unwrap_or_default(),
                            ));
                        }
                    }
                    "dag:const" | "const" => {
                        let node = DagNode::Const {
                            ty: attrs.get("type").cloned().unwrap_or_default(),
                            value: attrs.get("value").cloned().unwrap_or_default(),
                        };
                        if let Some(stack) = node_stack.last_mut() {
                            stack.push(node);
                        }
                    }
                    "dag:local.get" | "local.get" => {
                        let node = DagNode::LocalGet {
                            name: attrs.get("name").cloned().unwrap_or_default(),
                        };
                        if let Some(stack) = node_stack.last_mut() {
                            stack.push(node);
                        }
                    }
                    _ => {}
                }
                let _ = is_empty;
            }
            Ok(Event::End(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if name == "dag:function" || name == "function" {
                    if let Some(mut func) = current_func.take() {
                        if let Some(body) = node_stack.pop() {
                            func.body = body;
                        }
                        if let Some(m) = module.as_mut() {
                            m.functions.push(func);
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(anyhow::anyhow!("XML parse error: {}", e)),
            _ => {}
        }
        buf.clear();
    }

    module.context("No module found in XML")
}

/// Generate WAT from a Module.
pub fn generate_wat(module: &Module) -> String {
    let mut out = String::new();
    out.push_str(";; Generated by hologram-wat-transform (Rust)\n");
    out.push_str(";; SPDX-License-Identifier: LicenseRef-Sovereign-Leviathan-SL-AGPL3-001\n");
    out.push_str(&format!(";; Source: {}\n", module.name));
    out.push_str(";; DO NOT EDIT. Edit the XML DSL and re-run the transform.\n");
    out.push_str("(module\n");

    if let Some(pages) = module.memory_pages {
        out.push_str(&format!("  (memory {}", pages));
        if let Some(max) = module.memory_max {
            out.push_str(&format!(" {}", max));
        }
        out.push_str(")\n");
    }

    for func in &module.functions {
        out.push_str(&format!("  (func ${}", func.name));
        if func.export {
            out.push_str(&format!(" (export \"{}\")", func.name));
        }
        out.push_str("\n");
        for (name, ty) in &func.params {
            out.push_str(&format!("    (param ${} {})\n", name, ty));
        }
        for ty in &func.results {
            out.push_str(&format!("    (result {})\n", ty));
        }
        for (name, ty) in &func.locals {
            out.push_str(&format!("    (local ${} {})\n", name, ty));
        }
        for node in &func.body {
            emit_node(&mut out, node, 4);
        }
        out.push_str("  )\n");
    }

    out.push_str(")\n");
    out
}

fn emit_node(out: &mut String, node: &DagNode, indent: usize) {
    let pad = " ".repeat(indent);
    match node {
        DagNode::Const { ty, value } => {
            out.push_str(&format!("{}{}.const {}\n", pad, ty, value));
        }
        DagNode::LocalGet { name } => {
            out.push_str(&format!("{}local.get ${}\n", pad, name));
        }
        DagNode::LocalSet { name, body } => {
            for n in body {
                emit_node(out, n, indent);
            }
            out.push_str(&format!("{}local.set ${}\n", pad, name));
        }
        DagNode::Call { func, args } => {
            for arg in args {
                emit_node(out, arg, indent);
            }
            out.push_str(&format!("{}call ${}\n", pad, func));
        }
        DagNode::Add { ty, lhs, rhs } => {
            emit_node(out, lhs, indent);
            emit_node(out, rhs, indent);
            out.push_str(&format!("{}{}.add\n", pad, ty));
        }
        DagNode::Sub { ty, lhs, rhs } => {
            emit_node(out, lhs, indent);
            emit_node(out, rhs, indent);
            out.push_str(&format!("{}{}.sub\n", pad, ty));
        }
        DagNode::Mul { ty, lhs, rhs } => {
            emit_node(out, lhs, indent);
            emit_node(out, rhs, indent);
            out.push_str(&format!("{}{}.mul\n", pad, ty));
        }
        DagNode::Div { ty, lhs, rhs } => {
            emit_node(out, lhs, indent);
            emit_node(out, rhs, indent);
            let op = if ty == "i32" || ty == "i64" { "div_s" } else { "div" };
            out.push_str(&format!("{}{}.{}\n", pad, ty, op));
        }
        DagNode::And { ty, lhs, rhs } => {
            emit_node(out, lhs, indent);
            emit_node(out, rhs, indent);
            out.push_str(&format!("{}{}.and\n", pad, ty));
        }
        DagNode::Or { ty, lhs, rhs } => {
            emit_node(out, lhs, indent);
            emit_node(out, rhs, indent);
            out.push_str(&format!("{}{}.or\n", pad, ty));
        }
        DagNode::Xor { ty, lhs, rhs } => {
            emit_node(out, lhs, indent);
            emit_node(out, rhs, indent);
            out.push_str(&format!("{}{}.xor\n", pad, ty));
        }
        DagNode::Shl { ty, lhs, rhs } => {
            emit_node(out, lhs, indent);
            emit_node(out, rhs, indent);
            out.push_str(&format!("{}{}.shl\n", pad, ty));
        }
        DagNode::ShrU { ty, lhs, rhs } => {
            emit_node(out, lhs, indent);
            emit_node(out, rhs, indent);
            out.push_str(&format!("{}{}.shr_u\n", pad, ty));
        }
        DagNode::ShrS { ty, lhs, rhs } => {
            emit_node(out, lhs, indent);
            emit_node(out, rhs, indent);
            out.push_str(&format!("{}{}.shr_s\n", pad, ty));
        }
        DagNode::Rotr { ty, lhs, rhs } => {
            emit_node(out, lhs, indent);
            emit_node(out, rhs, indent);
            out.push_str(&format!("{}{}.rotr\n", pad, ty));
        }
        DagNode::Rotl { ty, lhs, rhs } => {
            emit_node(out, lhs, indent);
            emit_node(out, rhs, indent);
            out.push_str(&format!("{}{}.rotl\n", pad, ty));
        }
        DagNode::Load { ty, offset, align, addr } => {
            emit_node(out, addr, indent);
            out.push_str(&format!("{}{}.load offset={} align={}\n", pad, ty, offset, align));
        }
        DagNode::Store { ty, offset, align, addr, value } => {
            emit_node(out, addr, indent);
            emit_node(out, value, indent);
            out.push_str(&format!("{}{}.store offset={} align={}\n", pad, ty, offset, align));
        }
        DagNode::Block { label, body } => {
            out.push_str(&format!("{}block", pad));
            if let Some(l) = label {
                out.push_str(&format!(" ${}", l));
            }
            out.push_str("\n");
            for n in body {
                emit_node(out, n, indent + 2);
            }
            out.push_str(&format!("{}end\n", pad));
        }
        DagNode::Loop { label, body } => {
            out.push_str(&format!("{}loop", pad));
            if let Some(l) = label {
                out.push_str(&format!(" ${}", l));
            }
            out.push_str("\n");
            for n in body {
                emit_node(out, n, indent + 2);
            }
            out.push_str(&format!("{}end\n", pad));
        }
        DagNode::If { cond, then, els } => {
            for n in cond {
                emit_node(out, n, indent);
            }
            out.push_str(&format!("{}if\n", pad));
            for n in then {
                emit_node(out, n, indent + 2);
            }
            if let Some(els) = els {
                out.push_str(&format!("{}else\n", pad));
                for n in els {
                    emit_node(out, n, indent + 2);
                }
            }
            out.push_str(&format!("{}end\n", pad));
        }
        DagNode::Br { label } => {
            out.push_str(&format!("{}br ${}\n", pad, label));
        }
        DagNode::Return { value } => {
            if let Some(v) = value {
                emit_node(out, v, indent);
            }
            out.push_str(&format!("{}return\n", pad));
        }
        DagNode::Drop { value } => {
            emit_node(out, value, indent);
            out.push_str(&format!("{}drop\n", pad));
        }
        DagNode::Eqz { ty, operand } => {
            emit_node(out, operand, indent);
            out.push_str(&format!("{}{}.eqz\n", pad, ty));
        }
        DagNode::Eq { ty, lhs, rhs } => {
            emit_node(out, lhs, indent);
            emit_node(out, rhs, indent);
            out.push_str(&format!("{}{}.eq\n", pad, ty));
        }
        DagNode::Ne { ty, lhs, rhs } => {
            emit_node(out, lhs, indent);
            emit_node(out, rhs, indent);
            out.push_str(&format!("{}{}.ne\n", pad, ty));
        }
        DagNode::LtS { ty, lhs, rhs } => {
            emit_node(out, lhs, indent);
            emit_node(out, rhs, indent);
            out.push_str(&format!("{}{}.lt_s\n", pad, ty));
        }
        DagNode::LtU { ty, lhs, rhs } => {
            emit_node(out, lhs, indent);
            emit_node(out, rhs, indent);
            out.push_str(&format!("{}{}.lt_u\n", pad, ty));
        }
        DagNode::GtS { ty, lhs, rhs } => {
            emit_node(out, lhs, indent);
            emit_node(out, rhs, indent);
            out.push_str(&format!("{}{}.gt_s\n", pad, ty));
        }
        DagNode::GtU { ty, lhs, rhs } => {
            emit_node(out, lhs, indent);
            emit_node(out, rhs, indent);
            out.push_str(&format!("{}{}.gt_u\n", pad, ty));
        }
        DagNode::LeS { ty, lhs, rhs } => {
            emit_node(out, lhs, indent);
            emit_node(out, rhs, indent);
            out.push_str(&format!("{}{}.le_s\n", pad, ty));
        }
        DagNode::LeU { ty, lhs, rhs } => {
            emit_node(out, lhs, indent);
            emit_node(out, rhs, indent);
            out.push_str(&format!("{}{}.le_u\n", pad, ty));
        }
        DagNode::GeS { ty, lhs, rhs } => {
            emit_node(out, lhs, indent);
            emit_node(out, rhs, indent);
            out.push_str(&format!("{}{}.ge_s\n", pad, ty));
        }
        DagNode::GeU { ty, lhs, rhs } => {
            emit_node(out, lhs, indent);
            emit_node(out, rhs, indent);
            out.push_str(&format!("{}{}.ge_u\n", pad, ty));
        }
        DagNode::F32Abs { operand } => {
            emit_node(out, operand, indent);
            out.push_str(&format!("{}f32.abs\n", pad));
        }
        DagNode::F32Neg { operand } => {
            emit_node(out, operand, indent);
            out.push_str(&format!("{}f32.neg\n", pad));
        }
        DagNode::F32Sqrt { operand } => {
            emit_node(out, operand, indent);
            out.push_str(&format!("{}f32.sqrt\n", pad));
        }
        DagNode::F32Ceil { operand } => {
            emit_node(out, operand, indent);
            out.push_str(&format!("{}f32.ceil\n", pad));
        }
        DagNode::F32Floor { operand } => {
            emit_node(out, operand, indent);
            out.push_str(&format!("{}f32.floor\n", pad));
        }
        DagNode::F32Trunc { operand } => {
            emit_node(out, operand, indent);
            out.push_str(&format!("{}f32.trunc\n", pad));
        }
        DagNode::F32Nearest { operand } => {
            emit_node(out, operand, indent);
            out.push_str(&format!("{}f32.nearest\n", pad));
        }
        DagNode::F32Min { lhs, rhs } => {
            emit_node(out, lhs, indent);
            emit_node(out, rhs, indent);
            out.push_str(&format!("{}f32.min\n", pad));
        }
        DagNode::F32Max { lhs, rhs } => {
            emit_node(out, lhs, indent);
            emit_node(out, rhs, indent);
            out.push_str(&format!("{}f32.max\n", pad));
        }
        DagNode::F32Copysign { lhs, rhs } => {
            emit_node(out, lhs, indent);
            emit_node(out, rhs, indent);
            out.push_str(&format!("{}f32.copysign\n", pad));
        }
        DagNode::I32WrapI64 { operand } => {
            emit_node(out, operand, indent);
            out.push_str(&format!("{}i32.wrap_i64\n", pad));
        }
        DagNode::I32TruncF32S { operand } => {
            emit_node(out, operand, indent);
            out.push_str(&format!("{}i32.trunc_f32_s\n", pad));
        }
        DagNode::I32TruncF32U { operand } => {
            emit_node(out, operand, indent);
            out.push_str(&format!("{}i32.trunc_f32_u\n", pad));
        }
        DagNode::I32TruncF64S { operand } => {
            emit_node(out, operand, indent);
            out.push_str(&format!("{}i32.trunc_f64_s\n", pad));
        }
        DagNode::I32TruncF64U { operand } => {
            emit_node(out, operand, indent);
            out.push_str(&format!("{}i32.trunc_f64_u\n", pad));
        }
        DagNode::I64ExtendI32S { operand } => {
            emit_node(out, operand, indent);
            out.push_str(&format!("{}i64.extend_i32_s\n", pad));
        }
        DagNode::I64ExtendI32U { operand } => {
            emit_node(out, operand, indent);
            out.push_str(&format!("{}i64.extend_i32_u\n", pad));
        }
        DagNode::F32ConvertI32S { operand } => {
            emit_node(out, operand, indent);
            out.push_str(&format!("{}f32.convert_i32_s\n", pad));
        }
        DagNode::F32ConvertI32U { operand } => {
            emit_node(out, operand, indent);
            out.push_str(&format!("{}f32.convert_i32_u\n", pad));
        }
        DagNode::F32ConvertI64S { operand } => {
            emit_node(out, operand, indent);
            out.push_str(&format!("{}f32.convert_i64_s\n", pad));
        }
        DagNode::F32ConvertI64U { operand } => {
            emit_node(out, operand, indent);
            out.push_str(&format!("{}f32.convert_i64_u\n", pad));
        }
        DagNode::F32DemoteF64 { operand } => {
            emit_node(out, operand, indent);
            out.push_str(&format!("{}f32.demote_f64\n", pad));
        }
        DagNode::F64ConvertI32S { operand } => {
            emit_node(out, operand, indent);
            out.push_str(&format!("{}f64.convert_i32_s\n", pad));
        }
        DagNode::F64ConvertI32U { operand } => {
            emit_node(out, operand, indent);
            out.push_str(&format!("{}f64.convert_i32_u\n", pad));
        }
        DagNode::F64ConvertI64S { operand } => {
            emit_node(out, operand, indent);
            out.push_str(&format!("{}f64.convert_i64_s\n", pad));
        }
        DagNode::F64ConvertI64U { operand } => {
            emit_node(out, operand, indent);
            out.push_str(&format!("{}f64.convert_i64_u\n", pad));
        }
        DagNode::F64PromoteF32 { operand } => {
            emit_node(out, operand, indent);
            out.push_str(&format!("{}f64.promote_f32\n", pad));
        }
        DagNode::I32ReinterpretF32 { operand } => {
            emit_node(out, operand, indent);
            out.push_str(&format!("{}i32.reinterpret_f32\n", pad));
        }
        DagNode::I64ReinterpretF64 { operand } => {
            emit_node(out, operand, indent);
            out.push_str(&format!("{}i64.reinterpret_f64\n", pad));
        }
        DagNode::F32ReinterpretI32 { operand } => {
            emit_node(out, operand, indent);
            out.push_str(&format!("{}f32.reinterpret_i32\n", pad));
        }
        DagNode::F64ReinterpretI64 { operand } => {
            emit_node(out, operand, indent);
            out.push_str(&format!("{}f64.reinterpret_i64\n", pad));
        }
        DagNode::MemorySize => {
            out.push_str(&format!("{}memory.size\n", pad));
        }
        DagNode::MemoryGrow { operand } => {
            emit_node(out, operand, indent);
            out.push_str(&format!("{}memory.grow\n", pad));
        }
        DagNode::Unreachable => {
            out.push_str(&format!("{}unreachable\n", pad));
        }
        DagNode::Nop => {
            out.push_str(&format!("{}nop\n", pad));
        }
        DagNode::Select { cond, val1, val2 } => {
            emit_node(out, val1, indent);
            emit_node(out, val2, indent);
            emit_node(out, cond, indent);
            out.push_str(&format!("{}select\n", pad));
        }
    }
}

/// Transform XML to WAT, validating with the `wat` crate.
pub fn transform(xml: &str) -> Result<String> {
    let module = parse_xml(xml)?;
    let wat_text = generate_wat(&module);
    // Validate: parse WAT to WASM binary. If invalid, fail.
    wat::parse_str(&wat_text).context("Generated WAT failed to parse")?;
    Ok(wat_text)
}

/// Transform XML file to WAT file.
pub fn transform_file(xml_path: &str, wat_path: &str) -> Result<()> {
    let xml = std::fs::read_to_string(xml_path)
        .with_context(|| format!("Failed to read {}", xml_path))?;
    let wat_text = transform(&xml)?;
    std::fs::write(wat_path, wat_text)
        .with_context(|| format!("Failed to write {}", wat_path))?;
    Ok(())
}
