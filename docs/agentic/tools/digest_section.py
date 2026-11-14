#!/usr/bin/env python3
"""Print selected sections of the spec digest or screen inventory.

Agents read only the sections a task needs instead of whole files.

Usage:
  python3 -I docs/agentic/tools/digest_section.py top 4.4 5.3 7
  python3 -I docs/agentic/tools/digest_section.py P-05 D-03 inv:D

Selectors:
  top        digest title, reading notes and conventions (before section 1)
  N or N.M   a digest section, e.g. 4 (whole of section 4), 4.4, 5.3, 12
  X-NN       a screen-inventory entry, e.g. P-05, D-03, W-01
  inv:L      a lettered screen-inventory section, e.g. inv:A, inv:D
Unknown selectors are reported on stderr and the exit code is 1.
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DIGEST = ROOT / "spec-digest.md"
INVENTORY = ROOT / "screen-inventory.md"


def headings(lines):
    out = []
    for i, line in enumerate(lines):
        m = re.match(r"^(#{1,6})\s+(.*)$", line)
        if m:
            out.append((i, len(m.group(1)), m.group(2).strip()))
    return out


def section(lines, heads, match):
    for idx, (i, level, text) in enumerate(heads):
        if match(level, text):
            end = len(lines)
            for j, lvl, _ in heads[idx + 1:]:
                if lvl <= level:
                    end = j
                    break
            return "\n".join(lines[i:end]).rstrip()
    return None


def main(args):
    if not args:
        print(__doc__)
        return 1
    dl = DIGEST.read_text(encoding="utf-8").splitlines()
    il = INVENTORY.read_text(encoding="utf-8").splitlines()
    dh, ih = headings(dl), headings(il)
    failed = False
    for sel in args:
        sel = sel.strip().lstrip("§")
        text = None
        if sel == "top":
            first = next((i for i, lvl, t in dh if lvl == 2 and re.match(r"1\.", t)), len(dl))
            text = "\n".join(dl[:first]).rstrip()
        elif re.fullmatch(r"\d+(\.\d+)?", sel):
            if "." in sel:
                text = section(dl, dh, lambda lvl, t: re.match(re.escape(sel) + r"(\s|$)", t) is not None)
            else:
                text = section(dl, dh, lambda lvl, t: lvl == 2 and re.match(re.escape(sel) + r"\.\s", t) is not None)
        elif re.fullmatch(r"[A-Z]-\d{2}", sel):
            text = section(il, ih, lambda lvl, t: t.startswith(sel + " "))
        elif sel.startswith("inv:"):
            letter = sel[4:]
            text = section(il, ih, lambda lvl, t: lvl == 2 and t.startswith(letter + ". "))
        if text is None:
            print(f"digest_section: no section matches '{sel}'", file=sys.stderr)
            failed = True
            continue
        print(f"<!-- {sel} -->\n{text}\n")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
