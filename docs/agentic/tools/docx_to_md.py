#!/usr/bin/env python3
"""Convert .docx files to compact Markdown (headings, lists, tables), stdlib only.

Usage: python3 docx_to_md.py <input.docx> <output.md>
"""
import re
import sys
import zipfile
import xml.etree.ElementTree as ET

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


def para_text(p):
    out = []
    for node in p.iter():
        if node.tag == W + "t" and node.text:
            out.append(node.text)
        elif node.tag == W + "tab":
            out.append(" ")
        elif node.tag in (W + "br", W + "cr"):
            out.append(" ")
    return re.sub(r"\s+", " ", "".join(out)).strip()


def para_style(p):
    ppr = p.find(W + "pPr")
    if ppr is None:
        return "", None
    style = ppr.find(W + "pStyle")
    name = style.get(W + "val") if style is not None else ""
    num = ppr.find(W + "numPr")
    level = None
    if num is not None:
        ilvl = num.find(W + "ilvl")
        level = int(ilvl.get(W + "val")) if ilvl is not None else 0
    return name, level


def heading_prefix(style):
    s = style.lower()
    if s == "title":
        return "# "
    if s == "subtitle":
        return "_"
    m = re.match(r"heading\s*(\d)", s)
    if m:
        return "#" * min(int(m.group(1)) + 1, 6) + " "
    return ""


def cell_text(tc):
    parts = [para_text(p) for p in tc.iter(W + "p")]
    return " / ".join(t for t in parts if t).replace("|", "\\|")


def table_md(tbl):
    rows = []
    for tr in tbl.findall(W + "tr"):
        rows.append([cell_text(tc) for tc in tr.findall(W + "tc")])
    rows = [r for r in rows if any(r)]
    if not rows:
        return ""
    width = max(len(r) for r in rows)
    rows = [r + [""] * (width - len(r)) for r in rows]
    lines = ["| " + " | ".join(rows[0]) + " |", "|" + "---|" * width]
    lines += ["| " + " | ".join(r) + " |" for r in rows[1:]]
    return "\n".join(lines)


def convert(path):
    with zipfile.ZipFile(path) as z:
        root = ET.fromstring(z.read("word/document.xml"))
    body = root.find(W + "body")
    out = []
    for el in body:
        if el.tag == W + "p":
            text = para_text(el)
            if not text:
                if el.find(".//" + W + "drawing") is not None:
                    out.append("[figure]")
                continue
            style, level = para_style(el)
            prefix = heading_prefix(style)
            if prefix == "_":
                out.append(f"_{text}_")
            elif prefix:
                out.append(prefix + text)
            elif level is not None:
                out.append("  " * level + "- " + text)
            else:
                out.append(text)
        elif el.tag == W + "tbl":
            md = table_md(el)
            if md:
                out.append(md)
    # Blank line between blocks, but keep list items tight.
    result = []
    for i, line in enumerate(out):
        is_list = line.lstrip().startswith("- ")
        prev_list = i > 0 and out[i - 1].lstrip().startswith("- ")
        if result and not (is_list and prev_list):
            result.append("")
        result.append(line)
    return "\n".join(result) + "\n"


if __name__ == "__main__":
    src, dst = sys.argv[1], sys.argv[2]
    md = convert(src)
    with open(dst, "w", encoding="utf-8") as f:
        f.write(f"<!-- Generated from {src.split('/')[-1]} by docx_to_md.py. Do not edit; re-run the script. -->\n\n")
        f.write(md)
    print(f"{dst}: {len(md):,} chars")
