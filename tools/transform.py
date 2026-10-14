#!/usr/bin/env python3
# Copyright (C) 2026 Ahmad Ali Parr
# SPDX-License-Identifier: LicenseRef-Sovereign-Leviathan-SL-AGPL3-001
# NO-AI-TRAINING: This file may not be used to train machine learning models.
"""Transform XML DAG DSL to pure WAT via XSLT.

Usage:
    python3 tools/transform.py dsl/ wat/
    python3 tools/transform.py dsl/sha256.xml wat/sha256.wat

Requires: lxml (pip install lxml) or xsltproc.
"""
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
XSLT = ROOT / "xslt" / "dag-to-wat.xsl"


def transform_with_xsltproc(xml_path, out_path):
    result = subprocess.run(
        ["xsltproc", str(XSLT), str(xml_path)],
        capture_output=True, text=True)
    if result.returncode != 0:
        raise RuntimeError(f"xsltproc failed: {result.stderr}")
    out_path.write_text(result.stdout)
    return result.stdout


def transform_with_lxml(xml_path, out_path):
    from lxml import etree
    xml = etree.parse(str(xml_path))
    xslt = etree.parse(str(XSLT))
    transform = etree.XSLT(xslt)
    result = transform(xml)
    out_path.write_text(str(result))
    return str(result)


def transform_one(xml_path, out_path):
    try:
        text = transform_with_xsltproc(xml_path, out_path)
    except FileNotFoundError:
        text = transform_with_lxml(xml_path, out_path)
    lines = text.count("\n")
    print(f"{xml_path.name} -> {out_path.name} ({lines} lines WAT)")
    return text


def main():
    if len(sys.argv) < 3:
        print(__doc__)
        sys.exit(1)

    src = pathlib.Path(sys.argv[1])
    dst = pathlib.Path(sys.argv[2])

    if src.is_file():
        dst.parent.mkdir(parents=True, exist_ok=True)
        transform_one(src, dst)
    else:
        dst.mkdir(parents=True, exist_ok=True)
        for xml_file in sorted(src.glob("*.xml")):
            out = dst / (xml_file.stem + ".wat")
            transform_one(xml_file, out)


if __name__ == "__main__":
    raise SystemExit(main())
