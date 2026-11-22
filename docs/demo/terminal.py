#!/usr/bin/env python3
"""Renders the real `origami --review` output of docs/demo/ into an animated
SVG terminal (light and dark) for the README and the website.

    cargo build --release && python3 docs/demo/terminal.py

The command is typed, the output appears line by line, holds, and loops.
Colors follow docs/brand.md (Tokyo Night): added = valley blue, removed =
mountain red, rule IDs cyan.
"""
import html
import os
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DEMO = ROOT / "docs/demo"
FILE = "zcl_flight_check.clas.abap"
ARGS = ["--no-config", "--line-width", "80", "--diagnostics", "human", "--review", FILE]

THEMES = {
    # Tokyo Night Light and Tokyo Night (docs/brand.md).
    "light": dict(page="#d5d6db", sheet="#e6e7ed", ink="#343b58", dim="#5f6272",
                  red="#8c4351", blue="#34548a", cyan="#0f4b6e", red_wash="#dbd3da",
                  blue_wash="#d1d5e1", edge="#a8aab6", fold="#a8aab6"),
    "dark": dict(page="#16161e", sheet="#1a1b26", ink="#c0caf5", dim="#828bb8",
                 red="#f7768e", blue="#7aa2f7", cyan="#7dcfff", red_wash="#37222c",
                 blue_wash="#20303b", edge="#3b4261", fold="#3b4261"),
}
FONT = "ui-monospace, SFMono-Regular, Menlo, Consolas, 'Liberation Mono', monospace"
SIZE = 12
CW = SIZE * 0.6          # monospace advance
LH = 18                  # line height
PAD_X, PAD_Y = 24, 28
LOOP = 14.0              # seconds per loop
TYPE_START, TYPE_END = 0.4, 1.8
OUT_START, OUT_STEP = 2.3, 0.07


def run():
    env = dict(os.environ, CLICOLOR_FORCE="1")
    env.pop("NO_COLOR", None)
    env.pop("CLAUDECODE", None)
    p = subprocess.run([str(ROOT / "target/release/origami"), *ARGS], cwd=DEMO,
                       capture_output=True, text=True, env=env)
    return (p.stdout + p.stderr).rstrip("\n").split("\n")


SGR = re.compile(r"\x1b\[([0-9;]*)m")


def segments(line):
    """[(text, codes)] for one line with ANSI SGR codes."""
    out, codes, pos = [], set(), 0
    for m in SGR.finditer(line):
        if m.start() > pos:
            out.append((line[pos:m.start()], frozenset(codes)))
        new = m.group(1).split(";") if m.group(1) else ["0"]
        if new == ["0"]:
            codes = set()
        else:
            codes |= set(new)
        pos = m.end()
    if pos < len(line):
        out.append((line[pos:], frozenset(codes)))
    return out


def style(codes, t):
    fill, weight, wash = t["ink"], "400", None
    if "2" in codes:
        fill = t["dim"]
    if "31" in codes:
        fill = t["red"]
    if "32" in codes:
        fill = t["blue"]
    if "36" in codes:
        fill, weight = t["cyan"], "600"
    if "1" in codes:
        weight = "700"
    if "7" in codes:
        wash = t["red_wash"] if "31" in codes else t["blue_wash"]
    return fill, weight, wash


def pct(seconds):
    return f"{seconds / LOOP * 100:.2f}%"


def render(theme, lines):
    t = THEMES[theme]
    cmd = "origami --review " + FILE
    rows = [None] + lines            # row 0: the prompt
    cols = max(len(SGR.sub("", l)) for l in lines + ["$ " + cmd])
    width = round(PAD_X * 2 + cols * CW)
    height = round(PAD_Y * 2 + len(rows) * LH)
    css, body = [], []
    # Typing the command.
    n = len(cmd)
    css.append(f"@keyframes type {{0%,{pct(TYPE_START)}{{width:0}}"
               f"{pct(TYPE_END)},97%{{width:{n * CW:.1f}px}}100%{{width:0}}}}")
    css.append(f".cmd{{animation:type {LOOP}s steps({n}) infinite}}")
    y0 = PAD_Y + SIZE
    body.append(f'<text x="{PAD_X}" y="{y0}" fill="{t["blue"]}" font-weight="700">$</text>')
    body.append(f'<clipPath id="c-{theme}"><rect class="cmd" x="{PAD_X + 2 * CW}" y="{y0 - SIZE}" '
                f'width="0" height="{LH}"/></clipPath>')
    body.append(f'<text x="{PAD_X + 2 * CW}" y="{y0}" fill="{t["ink"]}" clip-path="url(#c-{theme})">'
                f'{html.escape(cmd)}</text>')
    for i, line in enumerate(lines, start=1):
        start = OUT_START + i * OUT_STEP
        css.append(f"@keyframes l{i}{{0%,{pct(start)}{{opacity:0}}"
                   f"{pct(start + 0.01)},97%{{opacity:1}}100%{{opacity:0}}}}")
        css.append(f".l{i}{{animation:l{i} {LOOP}s infinite}}")
        y = y0 + i * LH
        parts, col = [], 0
        for text, codes in segments(line):
            fill, weight, wash = style(codes, t)
            x = PAD_X + col * CW
            if wash:
                parts.append(f'<rect x="{x:.1f}" y="{y - SIZE + 1}" width="{len(text) * CW:.1f}" '
                             f'height="{LH - 2}" rx="2" fill="{wash}"/>')
            if text.strip():
                parts.append(f'<text x="{x:.1f}" y="{y}" fill="{fill}" font-weight="{weight}" '
                             f'xml:space="preserve">{html.escape(text)}</text>')
            col += len(text)
        body.append(f'<g class="l{i}">' + "".join(parts) + "</g>")
    reduced = ("@media (prefers-reduced-motion: reduce){.cmd{animation:none;width:"
               f"{n * CW:.1f}px}}" + "".join(f".l{i}{{animation:none}}" for i in range(1, len(lines) + 1))
               + "}")
    fold = 22
    return f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" role="img" aria-labelledby="t-{theme} d-{theme}" font-family="{FONT}" font-size="{SIZE}">
<title id="t-{theme}">origami --review</title>
<desc id="d-{theme}">The review view of origami on an ABAP class: the method signature, a method call and a SELECT are broken onto several lines, each with a note naming the style rule and the column the line would have ended at.</desc>
<style>{''.join(css)}{reduced}</style>
<rect width="{width}" height="{height}" rx="6" fill="{t["sheet"]}" stroke="{t["edge"]}"/>
<path d="M{width - fold} 0.5 V{fold} H{width - 0.5} Z" fill="{t["fold"]}"/>
<path d="M{width - fold} 0.5 L{width - 0.5} {fold}" stroke="{t["blue"]}" stroke-dasharray="3 2"/>
{chr(10).join(body)}
</svg>
"""


if __name__ == "__main__":
    lines = run()
    for theme in THEMES:
        suffix = "" if theme == "light" else "-dark"
        out = ROOT / f"docs/assets/review-demo{suffix}.svg"
        out.write_text(render(theme, lines))
        print(out.relative_to(ROOT), f"{len(lines)} lines")
