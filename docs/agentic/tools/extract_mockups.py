#!/usr/bin/env python3
"""Extract the design canvas mockups (.dc.html) into one compact Markdown outline.

Keeps: screen title and size, tweak props (alternate states), headings, visible
text, buttons, links (with targets), inputs and labels, conditional blocks
(sc-if) and repeats (sc-for), and the renderVals() script with long CSS
"scene" gradients collapsed. Drops: styles, SVG paths, layout markup.

Usage: python3 extract_mockups.py <project_dir> <output.md>
"""
import html
import json
import os
import re
import sys
from html.parser import HTMLParser

SKIP_TAGS = {"style", "helmet", "head", "script", "title"}
COLLECT_TAGS = {"button", "a", "label", "h1", "h2", "h3", "legend", "th"}
VOID = {"input", "img", "br", "meta", "link", "hr"}


def clean(s):
    return re.sub(r"\s+", " ", s).strip()


class Outline(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.lines = []
        self.depth = 0          # indentation from sc-if / sc-for
        self.skip = 0           # inside style/svg/etc.
        self.collect = []       # stack of [tag, attrs, texts]
        self.in_xdc = False

    def emit(self, text):
        if text:
            self.lines.append("  " * self.depth + text)

    def handle_starttag(self, tag, attrs):
        a = dict(attrs)
        if tag == "x-dc":
            self.in_xdc = True
            return
        if not self.in_xdc:
            return
        if tag in SKIP_TAGS:
            self.skip += 1
            return
        if tag == "svg":
            label = a.get("aria-label")
            if a.get("role") == "img" and label:
                self._text(f"[icon: {label}]")
            if tag not in VOID:
                self.skip += 1
            return
        if self.skip:
            return
        if tag == "sc-if":
            self.emit(f"▸ if {a.get('value', '').strip('{} ')}:")
            self.depth += 1
        elif tag == "sc-for":
            self.emit(f"▸ for each {a.get('as', 'item')} in {a.get('list', '').strip('{} ')}:")
            self.depth += 1
        elif tag == "dc-import":
            self.emit(f"[component {a.get('name')}]")
        elif tag == "input":
            kind = a.get("type", "text")
            bits = [f"input:{kind}"]
            for key in ("placeholder", "value"):
                if a.get(key):
                    bits.append(f'{key}="{a[key]}"')
            if "checked" in a:
                bits.append("checked")
            self._text("[" + " ".join(bits) + "]")
        elif tag == "textarea":
            self.collect.append(["textarea", a, []])
        elif tag in COLLECT_TAGS:
            self.collect.append([tag, a, []])
        elif tag in ("div", "section", "article", "header", "main", "aside", "nav", "form", "fieldset", "p", "li", "tr") and a.get("aria-label"):
            self.emit(f"<{tag} “{a['aria-label']}”>")

    def handle_endtag(self, tag):
        if tag == "x-dc":
            self.in_xdc = False
            return
        if not self.in_xdc:
            return
        if tag in SKIP_TAGS or tag == "svg":
            if self.skip:
                self.skip -= 1
            return
        if self.skip:
            return
        if tag in ("sc-if", "sc-for"):
            self.depth = max(0, self.depth - 1)
            return
        if self.collect and self.collect[-1][0] == tag:
            t, a, texts = self.collect.pop()
            text = clean(" ".join(texts))
            aria = a.get("aria-label")
            if t in ("h1", "h2", "h3"):
                line = "#" * int(t[1]) + " " + text
            elif t == "button":
                line = f"[button: {text or aria}]" + (f" (aria: {aria})" if aria and text and aria != text else "")
                if a.get("onclick") or a.get("onClick"):
                    line += f" → {(a.get('onclick') or a.get('onClick')).strip('{} ')}"
            elif t == "a":
                line = f"[link → {a.get('href', '')}: {text or aria}]"
            elif t == "label":
                line = f"label: {text}"
            elif t == "textarea":
                line = f"[textarea: {text}]"
            else:
                line = f"{t}: {text}"
            if self.collect:
                self.collect[-1][2].append(line)
            else:
                self.emit(line)

    def _text(self, text):
        if self.collect:
            self.collect[-1][2].append(text)
        else:
            self.emit(text)

    def handle_data(self, data):
        if not self.in_xdc or self.skip:
            return
        t = clean(data)
        if t:
            self._text(t)


def collapse_scenes(js):
    # Long CSS gradient strings stand in for photos; collapse them.
    js = re.sub(r"'[^'\n]{80,}gradient[^'\n]*'", "'<scene>'", js)
    js = re.sub(r"'[^'\n]*gradient\([^'\n]{80,}'", "'<scene>'", js)
    return js


def extract(path):
    src = open(path, encoding="utf-8").read()
    title = re.search(r"<title>(.*?)</title>", src, re.S)
    title = html.unescape(title.group(1).strip()) if title else os.path.basename(path)
    m = re.search(r"<script type=\"text/x-dc\"[^>]*data-props='(.*?)'>(.*?)</script>", src, re.S)
    props, script = {}, ""
    if m:
        try:
            props = json.loads(html.unescape(m.group(1)))
        except json.JSONDecodeError:
            props = {}
        script = collapse_scenes(m.group(2).strip())
    p = Outline()
    p.feed(src)
    # Drop consecutive duplicate lines.
    lines = [l for i, l in enumerate(p.lines) if i == 0 or l != p.lines[i - 1]]
    return title, props, lines, script


def main(project, out):
    canvas = json.load(open(os.path.join(project, "canvas.json"), encoding="utf-8"))
    order = canvas.get("order", [])
    boards = canvas.get("boards", {})
    parts = ["<!-- Generated from design/mvp-ui/project by extract_mockups.py. Do not edit; re-run the script. -->", "",
             "# Locale MVP UI: screen outlines", "",
             "Each screen lists its tweaks (switches between alternate states), an outline of what is on screen "
             "(▸ if = shown only in some states; ▸ for each = repeated from data), and its renderVals() script, "
             "which holds the sample data and state logic. `'<scene>'` = a placeholder photo.", "",
             "## Canvas notes", ""]
    for note in canvas.get("notes", {}).values():
        if note.get("kind") is None and note.get("text"):
            parts += ["```", note["text"], "```", ""]
    parts += ["## Screens", "", "| File | Title on canvas | Size |", "|---|---|---|"]
    for name in order:
        b = boards.get(name, {})
        parts.append(f"| {name} | {b.get('title', '')} | {b.get('w')}×{b.get('h')}{' (page)' if b.get('expand') else ''} |")
    parts.append("")
    for name in order:
        path = os.path.join(project, name)
        if not os.path.exists(path):
            continue
        title, props, lines, script = extract(path)
        b = boards.get(name, {})
        parts += [f"---", "", f"## {b.get('title', name)}", "", f"File: `{name}` · Page title: {title}", ""]
        tweaks = {k: v for k, v in props.items() if not k.startswith("$")}
        if tweaks:
            parts.append("Tweaks:")
            for k, v in tweaks.items():
                opts = v.get("options")
                parts.append(f"- `{k}` ({v.get('editor')}): " + (f"{' | '.join(map(str, opts))}" if opts else "") + f" — default `{v.get('default')}`")
            parts.append("")
        parts += ["Outline:", "", "```"] + lines + ["```", ""]
        if script:
            parts += ["Script (data and state):", "", "```js", script, "```", ""]
    md = "\n".join(parts) + "\n"
    with open(out, "w", encoding="utf-8") as f:
        f.write(md)
    print(f"{out}: {len(md):,} chars")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
