// SPDX-License-Identifier: LicenseRef-Sovereign-Leviathan-SL-AGPL3-001
//! CLI for hologram-wat-transform.
//!
//! Usage:
//!   hologram-wat dsl/sha256.xml wat/sha256.wat
//!   hologram-wat dsl/ wat/   (transform all .xml in dir)

use hologram_wat_transform::transform_file;
use std::env;
use std::path::Path;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <xml-file-or-dir> <wat-file-or-dir>", args[0]);
        process::exit(1);
    }

    let src = Path::new(&args[1]);
    let dst = Path::new(&args[2]);

    if src.is_file() {
        if let Err(e) = transform_file(&args[1], &args[2]) {
            eprintln!("Error: {:#}", e);
            process::exit(1);
        }
        println!("{} -> {}", args[1], args[2]);
    } else if src.is_dir() {
        std::fs::create_dir_all(dst).unwrap_or(());
        let mut count = 0;
        let mut failed = 0;
        for entry in std::fs::read_dir(src).unwrap().flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "xml").unwrap_or(false) {
                let stem = path.file_stem().unwrap().to_string_lossy();
                let out = dst.join(format!("{}.wat", stem));
                match transform_file(
                    path.to_str().unwrap(),
                    out.to_str().unwrap(),
                ) {
                    Ok(()) => {
                        println!("{} -> {}", path.display(), out.display());
                        count += 1;
                    }
                    Err(e) => {
                        eprintln!("FAIL {}: {:#}", path.display(), e);
                        failed += 1;
                    }
                }
            }
        }
        println!("Transformed {} files, {} failed", count, failed);
        if failed > 0 {
            process::exit(1);
        }
    } else {
        eprintln!("Not found: {}", args[1]);
        process::exit(1);
    }
}
