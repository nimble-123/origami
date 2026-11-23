#!/usr/bin/env python3
"""Builds the origami diagrams (light and dark HTML) from one description.

Palette and type: docs/brand.md. Run: python3 docs/diagrams/build.py
Then export the SVGs for README and website (see docs/diagrams/README.md).
"""
import json
import math
from pathlib import Path

HERE = Path(__file__).parent
DATA = json.loads((HERE.parent / "benchmark-data.json").read_text())
COMPARE = json.loads((HERE.parent / "benchmark-compare-data.json").read_text())

SKINS = {
    # Tokyo Night Light by day, Tokyo Night by night (docs/brand.md).
    "light": dict(paper="#d5d6db", sheet="#e6e7ed", ink="#343b58", muted="#565a6e",
                  soft="#5f6272", crease="#a8aab6", accent="#34548a",
                  accent_tint="rgba(52,84,138,0.10)", ink_rgb="52,59,88",
                  series1="#7c7f8f"),
    "dark": dict(paper="#16161e", sheet="#1a1b26", ink="#c0caf5", muted="#a9b1d6",
                 soft="#828bb8", crease="#3b4261", accent="#7aa2f7",
                 accent_tint="rgba(122,162,247,0.12)", ink_rgb="192,202,245",
                 series1="#737aa2"),
}
SANS = "'Schibsted Grotesk', system-ui, sans-serif"
MONO = "'Martian Mono', ui-monospace, monospace"
FONTS = ('<link href="https://fonts.googleapis.com/css2?family=Schibsted+Grotesk:'
         'wght@400;500;600;700;800&family=Martian+Mono:wght@400;500;600&display=swap"'
         ' rel="stylesheet">')


def page(slug, title, desc, width, height, body, s, caption):
    return f"""<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{title}</title>
  {FONTS}
  <style>
    *, *::before, *::after {{ box-sizing: border-box; margin: 0; padding: 0; }}
    body {{ font-family: {SANS}; background: {s['paper']}; color: {s['ink']};
           min-height: 100vh; display: flex; align-items: center; justify-content: center;
           padding: 3rem 2rem; }}
    .frame {{ max-width: 1200px; width: 100%; }}
    .diagram-container {{ width: 100%; overflow-x: auto; }}
    h1 {{ font-size: clamp(1.5rem, 2.4vw + 0.75rem, 2rem); font-weight: 800;
          letter-spacing: -0.02em; line-height: 1.15; margin-bottom: 1.5rem; }}
    p.caption {{ color: {s['muted']}; font-size: 0.9rem; margin-top: 1rem; max-width: 68ch; }}
    svg {{ width: 100%; min-width: {width}px; display: block; }}
    @media print {{ .diagram-container {{ overflow-x: visible; }} svg {{ min-width: 0; }} }}
  </style>
</head>
<body>
  <div class="frame">
    <h1>{title}</h1>
    <div class="diagram-container">
      <svg viewBox="0 0 {width} {height}" xmlns="http://www.w3.org/2000/svg" role="img" aria-labelledby="{slug}-title {slug}-desc">
        <title id="{slug}-title">{title}</title>
        <desc id="{slug}-desc">{desc}</desc>
        <defs>
          <marker id="arrow" markerWidth="8" markerHeight="6" refX="7" refY="3" orient="auto"><polygon points="0 0, 8 3, 0 6" fill="{s['muted']}"/></marker>
          <marker id="arrow-accent" markerWidth="8" markerHeight="6" refX="7" refY="3" orient="auto"><polygon points="0 0, 8 3, 0 6" fill="{s['accent']}"/></marker>
          <marker id="arrow-link" markerWidth="8" markerHeight="6" refX="7" refY="3" orient="auto"><polygon points="0 0, 8 3, 0 6" fill="{s['accent']}"/></marker>
        </defs>
        <rect width="100%" height="100%" fill="{s['paper']}"/>
{body}
      </svg>
    </div>
    <p class="caption">{caption}</p>
  </div>
</body>
</html>
"""


def label(x, y, text, s, anchor="middle", color=None):
    """Arrow label: masked, Martian Mono 8px, sentence case."""
    w = round(len(text) * 8 * 0.75 + 12)
    w += (-w) % 4
    left = x - w / 2 if anchor == "middle" else (x - w + 6 if anchor == "end" else x - 6)
    return (f'        <rect x="{left:.0f}" y="{y - 9}" width="{w}" height="12" rx="2" fill="{s["paper"]}"/>\n'
            f'        <text x="{x}" y="{y}" fill="{color or s["soft"]}" font-size="8" font-family="{MONO}"'
            f' text-anchor="{anchor}" letter-spacing="0.04em">{text}</text>\n')


def node(x, y, w, h, name, sub, s, *, oval=False, focal=False, dashed=False):
    rx = h // 2 if oval else 4
    stroke = s["accent"] if focal else s["ink"]
    fill = s["accent_tint"] if focal else s["sheet"]
    dash = ' stroke-dasharray="4,3"' if dashed else ""
    cx = x + w // 2
    cy = y + h // 2
    return (f'        <rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{rx}" fill="{s["paper"]}"/>\n'
            f'        <rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{rx}" fill="{fill}" stroke="{stroke}" stroke-width="1"{dash}/>\n'
            f'        <text x="{cx}" y="{cy - 2}" fill="{s["ink"]}" font-size="12" font-weight="600" font-family="{SANS}" text-anchor="middle">{name}</text>\n'
            f'        <text x="{cx}" y="{cy + 14}" fill="{s["muted"]}" font-size="9" font-family="{MONO}" text-anchor="middle">{sub}</text>\n')


def pipeline(theme):
    s = SKINS[theme]
    slug = "pipeline" if theme == "light" else "pipeline-dark"
    m, a = s["muted"], s["accent"]
    arrows = "".join(
        f'        <line x1="{x1}" y1="204" x2="{x2}" y2="204" stroke="{m}" stroke-width="1" marker-end="url(#arrow)"/>\n'
        for x1, x2 in [(128, 166), (304, 342), (488, 526), (640, 658)])
    # crease frame around the self-check loop (a zone: painted first)
    body = (f'        <rect x="512" y="140" width="288" height="236" rx="4" fill="none" stroke="{s["crease"]}"'
            f' stroke-width="1" stroke-dasharray="6,4"/>\n'
            f'        <text x="520" y="132" fill="{s["soft"]}" font-size="8" font-family="{MONO}" letter-spacing="0.04em">self-check, after every format</text>\n')
    body += arrows
    # yes: diamond -> formatted file (accent: the path that writes)
    body += f'        <line x1="764" y1="204" x2="802" y2="204" stroke="{a}" stroke-width="1.2" marker-end="url(#arrow-accent)"/>\n'
    # no: diamond -> quarantine
    body += f'        <line x1="712" y1="248" x2="712" y2="310" stroke="{m}" stroke-width="1" marker-end="url(#arrow)"/>\n'
    # again: quarantine -> print (loop back)
    body += (f'        <path d="M644 340 H592 Q584 340 584 332 V234" fill="none" stroke="{m}"'
             f' stroke-width="1" marker-end="url(#arrow)"/>\n')
    # still different: quarantine -> left unchanged
    body += f'        <line x1="712" y1="368" x2="712" y2="406" stroke="{m}" stroke-width="1" stroke-dasharray="5,4" marker-end="url(#arrow)"/>\n'
    body += label(784, 192, "yes", s, color=a)
    body += label(722, 284, "no", s, anchor="start")
    body += label(572, 288, "print again", s, anchor="end")
    body += label(722, 398, "still different", s, anchor="start")
    body += node(24, 176, 104, 56, "Your file", "ABAP or CDS", s, oval=True)
    body += node(168, 176, 136, 56, "Lex", "lossless tokens", s)
    body += node(344, 176, 144, 56, "Parse and lay out", "layout tree", s)
    body += node(528, 176, 112, 56, "Print", "line width 120", s)
    # decision diamond (focal: the self-check)
    body += (f'        <polygon points="660,204 712,160 764,204 712,248" fill="{s["paper"]}"/>\n'
             f'        <polygon points="660,204 712,160 764,204 712,248" fill="{s["accent_tint"]}" stroke="{a}" stroke-width="1.2"/>\n'
             f'        <text x="712" y="200" fill="{s["ink"]}" font-size="12" font-weight="600" font-family="{SANS}" text-anchor="middle">Same</text>\n'
             f'        <text x="712" y="216" fill="{s["ink"]}" font-size="12" font-weight="600" font-family="{SANS}" text-anchor="middle">tokens?</text>\n')
    body += node(804, 176, 132, 56, "Formatted file", "written", s, oval=True)
    body += node(644, 312, 136, 56, "Quarantine unit", "keep as written", s)
    body += node(624, 408, 176, 48, "File left unchanged", "error X001", s, oval=True, dashed=True)
    title = "How origami keeps your code intact"
    desc = ("origami lexes the file, lays it out and prints it, then re-lexes the output and "
            "compares the tokens with the input; only identical results are written, a unit that "
            "differs is kept as written, and if it still differs the file stays unchanged.")
    caption = ("After formatting, origami reads its own output back and compares every token and "
               "comment with your file. Only whitespace, line breaks and letter case may differ.")
    body = f'        <g transform="translate(0,-100)">\n{body}        </g>\n'
    return page(slug, title, desc, 960, 380, body, s, caption)


def benchmark(theme):
    """Whole project: the 1,920 ABAP files, both tools, from bench-compare."""
    s = SKINS[theme]
    slug = "benchmark" if theme == "light" else "benchmark-dark"
    pr = COMPARE["project"]
    default = pr["default"]["warm_ms"] / 1000
    layout = pr["layout"]["warm_ms"] / 1000
    one = pr["origami_j1"]["warm"]["p50"] / 1000
    many = pr["origami"]["warm"]["p50"] / 1000
    cores = COMPARE["machine"]["cores"]
    rows = [("ABAP Cleaner, default profile", default, None),
            ("ABAP Cleaner, layout profile", layout, None),
            ("origami, one thread", one, "focal"),
            (f"origami, {cores} threads", many, "ratio")]
    x0, x1 = 264, 904
    ceil = math.ceil(default / 2) * 2
    px = (x1 - x0) / ceil
    body = ""
    top, bottom = 40, 56 + len(rows) * 56 - 8
    for t in range(0, ceil + 1, 2):
        x = round(x0 + t * px)
        stroke = f"rgba({s['ink_rgb']},0.25)" if t == 0 else f"rgba({s['ink_rgb']},0.08)"
        body += f'        <line x1="{x}" y1="{top}" x2="{x}" y2="{bottom}" stroke="{stroke}" stroke-width="{1 if t == 0 else 0.8}"/>\n'
        body += f'        <text x="{x}" y="{bottom + 20}" fill="{s["muted"]}" font-size="8" font-family="{MONO}" text-anchor="middle">{t} s</text>\n'
    for i, (name, v, kind) in enumerate(rows):
        y = 56 + i * 56
        w = max(2, round(v * px))
        focal = kind == "focal"
        fill = s["accent_tint"] if focal else f"rgba({s['ink_rgb']},0.06)"
        stroke = s["accent"] if focal else s["series1"]
        body += f'        <rect x="{x0}" y="{y}" width="{w}" height="32" fill="{s["paper"]}"/>\n'
        body += f'        <rect x="{x0}" y="{y}" width="{w}" height="32" fill="{fill}" stroke="{stroke}" stroke-width="1"/>\n'
        body += (f'        <text x="{x0 - 12}" y="{y + 20}" fill="{s["ink"]}" font-size="12" font-weight="600"'
                 f' font-family="{SANS}" text-anchor="end">{name}</text>\n')
        val = f"{v:.2f} s"
        if kind:
            val += f", {layout / v:.0f}× faster than the layout profile"
        body += (f'        <text x="{x0 + w + 10}" y="{y + 20}" fill="{s["accent"] if focal else s["muted"]}"'
                 f' font-size="9" font-family="{MONO}">{val}</text>\n')
    n = COMPARE["inputs"]["project"]["count"]
    body += (f'        <text x="584" y="{bottom + 44}" fill="{s["muted"]}" font-size="8" font-family="{MONO}" text-anchor="middle"'
             f' letter-spacing="0.04em">time for all {n:,} ABAP files of the test corpus, warm, same files for both tools</text>\n')
    title = "Formatting a whole project"
    desc = (f"Bar chart of the time to format the {n:,} ABAP files of the test corpus: ABAP Cleaner takes "
            f"{default:.2f} s with its default profile and {layout:.2f} s with its layout profile; origami takes "
            f"{one:.2f} s on one thread and {many:.2f} s on {cores} threads.")
    caption = (f"{COMPARE['machine']['cpu']}, {COMPARE['date']}. ABAP Cleaner runs on one thread; its layout "
               "profile keeps only the rules that change layout, its default profile also changes code. "
               "All numbers: docs/benchmark-abap-cleaner.md.")
    return page(slug, title, desc, 960, bottom + 60, body, s, caption)


def us_or_ms(v):
    """A time in milliseconds, written in µs below 1 ms."""
    if v < 1:
        u = v * 1000
        return f"{u:.1f} µs" if u < 10 else f"{u:.0f} µs"
    return f"{v:.1f} ms" if v < 10 else f"{v:.0f} ms"


def granularity(theme):
    """Dot plot on a log axis: origami and both ABAP Cleaner profiles, warm and cold,
    for one statement, one method and one file (from bench-compare)."""
    s = SKINS[theme]
    slug = "granularity" if theme == "light" else "granularity-dark"
    x0, x1, lo, hi = 210, 800, -3, 3              # 1 µs … 1 s
    xs = lambda v: round(x0 + (math.log10(v) - lo) / (hi - lo) * (x1 - x0))
    inp = COMPARE["inputs"]
    names = {"statement": "One statement", "method": "One method", "file": "One file"}

    def lines(n):
        return f"{n} line" + ("" if n == 1 else "s")

    groups = [
        ("Warm: one call in a running program, median of all inputs",
         [(names[g], {t: COMPARE["warm"][g][t]["time_us"]["p50"] / 1000 for t in ("origami", "layout", "default")})
          for g in ("statement", "method", "file")]),
        ("Cold: a new process per call, median-sized input",
         [(f"{names[g]}, {lines(inp[g]['lines_median'])}",
           {t: COMPARE["cold"][g][t]["time"]["p50"] for t in ("origami", "layout", "default")})
          for g in ("statement", "method", "file")]),
    ]
    body = ""
    y = 40
    row_ys = []
    layout_rows = []
    for header, rows in groups:
        body += (f'        <text x="24" y="{y}" fill="{s["ink"]}" font-size="12" font-weight="700" '
                 f'font-family="{SANS}">{header}</text>\n')
        y += 34
        for name, v in rows:
            layout_rows.append((y, name, v))
            row_ys.append(y)
            y += 44
        y += 8
    top, bottom = 52, y - 26
    for e in range(lo, hi + 1):
        x = xs(10 ** e)
        stroke = f"rgba({s['ink_rgb']},0.25)" if e == lo else f"rgba({s['ink_rgb']},0.08)"
        body += f'        <line x1="{x}" y1="{top}" x2="{x}" y2="{bottom}" stroke="{stroke}" stroke-width="{1 if e == lo else 0.8}"/>\n'
        tick = {-3: "1 µs", -2: "10 µs", -1: "100 µs", 0: "1 ms", 1: "10 ms", 2: "100 ms", 3: "1 s"}[e]
        body += f'        <text x="{x}" y="{bottom + 16}" fill="{s["muted"]}" font-size="8" font-family="{MONO}" text-anchor="middle">{tick}</text>\n'
    body += (f'        <text x="{(x0 + x1) // 2}" y="{bottom + 32}" fill="{s["muted"]}" font-size="8" font-family="{MONO}" '
             f'text-anchor="middle" letter-spacing="0.04em">time per call, log scale</text>\n')
    body += (f'        <text x="944" y="40" fill="{s["muted"]}" font-size="8" font-family="{MONO}" text-anchor="end" '
             f'letter-spacing="0.04em">slower than origami</text>\n')
    body += (f'        <text x="944" y="54" fill="{s["muted"]}" font-size="8" font-family="{MONO}" text-anchor="end" '
             f'letter-spacing="0.04em">layout · default</text>\n')
    connector = f"rgba({s['ink_rgb']},{0.55 if theme == 'light' else 0.40})"
    for y, name, v in layout_rows:
        a, b, c = xs(v["origami"]), xs(v["layout"]), xs(v["default"])
        body += (f'        <text x="{x0 - 24}" y="{y + 4}" fill="{s["ink"]}" font-size="11" font-weight="600" '
                 f'font-family="{SANS}" text-anchor="end">{name}</text>\n')
        body += f'        <line x1="{a}" y1="{y}" x2="{c}" y2="{y}" stroke="{connector}" stroke-width="1"/>\n'
        # ABAP Cleaner default profile: solid square; layout profile: hollow square; origami: accent dot.
        body += f'        <rect x="{c - 5}" y="{y - 5}" width="10" height="10" fill="{s["series1"]}" stroke="{s["ink"]}" stroke-width="1"/>\n'
        body += f'        <rect x="{b - 5}" y="{y - 5}" width="10" height="10" fill="{s["paper"]}" stroke="{s["series1"]}" stroke-width="1.5"/>\n'
        body += f'        <circle cx="{a}" cy="{y}" r="6" fill="{s["accent"]}" stroke="{s["ink"]}" stroke-width="1"/>\n'
        body += (f'        <text x="{a - 12}" y="{y + 4}" fill="{s["accent"]}" font-size="8" font-family="{MONO}" '
                 f'text-anchor="end">{us_or_ms(v["origami"])}</text>\n')
        body += (f'        <text x="{b}" y="{y - 11}" fill="{s["muted"]}" font-size="8" font-family="{MONO}" '
                 f'text-anchor="middle">{us_or_ms(v["layout"])}</text>\n')
        body += (f'        <text x="{c + 12}" y="{y + 4}" fill="{s["muted"]}" font-size="8" font-family="{MONO}">'
                 f'{us_or_ms(v["default"])}</text>\n')
        r1, r2 = v["layout"] / v["origami"], v["default"] / v["origami"]
        body += (f'        <text x="944" y="{y + 4}" fill="{s["ink"]}" font-size="9" font-family="{MONO}" '
                 f'text-anchor="end">{r1:.0f}× · {r2:.0f}×</text>\n')
    ly = bottom + 52
    body += f'        <line x1="24" y1="{ly - 16}" x2="944" y2="{ly - 16}" stroke="rgba({s["ink_rgb"]},0.10)" stroke-width="0.8"/>\n'
    body += f'        <circle cx="34" cy="{ly}" r="5" fill="{s["accent"]}" stroke="{s["ink"]}" stroke-width="1"/>\n'
    body += f'        <text x="46" y="{ly + 4}" fill="{s["muted"]}" font-size="9" font-family="{SANS}">origami</text>\n'
    body += f'        <rect x="112" y="{ly - 5}" width="10" height="10" fill="{s["paper"]}" stroke="{s["series1"]}" stroke-width="1.5"/>\n'
    body += f'        <text x="130" y="{ly + 4}" fill="{s["muted"]}" font-size="9" font-family="{SANS}">ABAP Cleaner, layout profile</text>\n'
    body += f'        <rect x="280" y="{ly - 5}" width="10" height="10" fill="{s["series1"]}" stroke="{s["ink"]}" stroke-width="1"/>\n'
    body += f'        <text x="298" y="{ly + 4}" fill="{s["muted"]}" font-size="9" font-family="{SANS}">ABAP Cleaner, default profile</text>\n'
    title = "One statement, one method, one file"
    def warm(g, t):
        return us_or_ms(COMPARE["warm"][g][t]["time_us"]["p50"] / 1000)

    def cold(g, t):
        return us_or_ms(COMPARE["cold"][g][t]["time"]["p50"])

    desc = ("Dot plot, log scale, of the median time per call for origami and ABAP Cleaner with its layout "
            f"and its default profile. Warm, one statement: origami {warm('statement', 'origami')}, ABAP Cleaner "
            f"{warm('statement', 'layout')} and {warm('statement', 'default')}; one file: origami "
            f"{warm('file', 'origami')}, ABAP Cleaner {warm('file', 'layout')} and {warm('file', 'default')}. "
            f"Cold, a new process per call, one file: origami {cold('file', 'origami')}, ABAP Cleaner "
            f"{cold('file', 'layout')} and {cold('file', 'default')}.")
    caption = (f"{COMPARE['machine']['cpu']}, {COMPARE['date']}. Warm: after the JIT compiler has warmed up. "
               "Cold: ABAP Cleaner through its Java API without the Eclipse start, so a lower bound. "
               "All numbers: docs/benchmark-abap-cleaner.md.")
    return page(slug, title, desc, 960, ly + 20, body, s, caption)


LATENCY_ROWS = [
    ("origami, in process", lambda d: d["per_file_in_process"]["format"]),
    ("origami, MCP request", lambda d: d["mcp_per_request"]),
    ("origami, WebAssembly", lambda d: d["wasm_per_file"]),
    ("origami, new process", lambda d: d["one_file_cold_cli"][0]["time"]),
    ("ABAP Cleaner, warm JVM", lambda d: d["abap_cleaner"]["per_file_warm"]),
    ("ABAP Cleaner, new JVM", lambda d: d["abap_cleaner"]["one_file_cold"]),
]


def num(v):
    """Milliseconds with sensible precision."""
    return f"{v:.2f}" if v < 1 else f"{v:.1f}" if v < 10 else f"{v:.0f}"


def latency(theme):
    """Dumbbell on a log axis: p50 (hollow) to p99 (solid) per way of use."""
    s = SKINS[theme]
    slug = "latency" if theme == "light" else "latency-dark"
    x0, x1, lo, hi = 280, 940, -2, 3          # 0.01 ms … 1000 ms
    xs = lambda v: round(x0 + (math.log10(v) - lo) / (hi - lo) * (x1 - x0))
    body = ""
    for e in range(lo, hi + 1):
        x = xs(10 ** e)
        stroke = f"rgba({s['ink_rgb']},0.25)" if e == lo else f"rgba({s['ink_rgb']},0.08)"
        body += f'        <line x1="{x}" y1="40" x2="{x}" y2="420" stroke="{stroke}" stroke-width="{1 if e == lo else 0.8}"/>\n'
        label = f"{10 ** e:g} ms"
        body += f'        <text x="{x}" y="440" fill="{s["muted"]}" font-size="8" font-family="{MONO}" text-anchor="middle">{label}</text>\n'
    body += (f'        <text x="{(x0 + x1) // 2}" y="456" fill="{s["muted"]}" font-size="8" font-family="{MONO}" '
             f'text-anchor="middle" letter-spacing="0.04em">milliseconds per call, log scale</text>\n')
    connector = f"rgba({s['ink_rgb']},{0.55 if theme == 'light' else 0.40})"
    for i, (name, get) in enumerate(LATENCY_ROWS):
        st = get(DATA)
        y = 76 + i * 64
        a, b = xs(st["p50"]), xs(st["p99"])
        body += f'        <line x1="{a}" y1="{y}" x2="{b}" y2="{y}" stroke="{connector}" stroke-width="1"/>\n'
        body += f'        <circle cx="{a}" cy="{y}" r="6" fill="{s["paper"]}" stroke="{s["muted"]}" stroke-width="1.5"/>\n'
        body += f'        <circle cx="{b}" cy="{y}" r="6" fill="{s["accent"]}" stroke="{s["ink"]}" stroke-width="1"/>\n'
        body += (f'        <text x="{a - 12}" y="{y + 4}" fill="{s["muted"]}" font-size="8" font-family="{MONO}" '
                 f'text-anchor="end">{num(st["p50"])}</text>\n')
        body += (f'        <text x="{b + 12}" y="{y + 4}" fill="{s["muted"]}" font-size="8" font-family="{MONO}">{num(st["p99"])}</text>\n')
        body += (f'        <text x="{x0 - 24}" y="{y + 4}" fill="{s["ink"]}" font-size="11" font-weight="600" '
                 f'font-family="{SANS}" text-anchor="end">{name}</text>\n')
    # Legend strip.
    body += f'        <line x1="30" y1="476" x2="930" y2="476" stroke="rgba({s["ink_rgb"]},0.10)" stroke-width="0.8"/>\n'
    body += f'        <circle cx="40" cy="493" r="5" fill="{s["paper"]}" stroke="{s["muted"]}" stroke-width="1.5"/>\n'
    body += f'        <text x="52" y="497" fill="{s["muted"]}" font-size="9" font-family="{SANS}">median (p50)</text>\n'
    body += f'        <circle cx="160" cy="493" r="5" fill="{s["accent"]}" stroke="{s["ink"]}" stroke-width="1"/>\n'
    body += f'        <text x="172" y="497" fill="{s["muted"]}" font-size="9" font-family="{SANS}">p99: 99 % of calls are faster</text>\n'
    body += (f'        <text x="930" y="497" fill="{s["muted"]}" font-size="8" font-family="{MONO}" text-anchor="end">'
             f'new process: median-sized file, 91 lines</text>\n')
    title = "How long one call takes"
    desc = ("Dumbbell chart, log scale, of the median and 99th percentile time per call: origami in process, "
            "through MCP, in WebAssembly and as a new process, and ABAP Cleaner with a warm and with a new JVM.")
    caption = (f"{DATA['machine']['cpu']}, {DATA['date']}, every corpus file (in process) or every fourth "
               "(MCP, WebAssembly); new process: 30 runs. All numbers: docs/benchmark.md.")
    return page(slug, title, desc, 960, 512, body, s, caption)


def scaling(theme):
    s = SKINS[theme]
    slug = "threads" if theme == "light" else "threads-dark"
    rows = [(r["threads"], r["time"]["p50"]) for r in DATA["corpus_by_threads"]]
    top = math.ceil(max(v for _, v in rows) / 200) * 200
    y0, y1 = 360, 40
    ys = lambda v: round(y0 - v / top * (y0 - y1))
    body = ""
    for t in range(0, top + 1, 200):
        y = ys(t)
        stroke = f"rgba({s['ink_rgb']},0.25)" if t == 0 else f"rgba({s['ink_rgb']},0.08)"
        body += f'        <line x1="96" y1="{y}" x2="920" y2="{y}" stroke="{stroke}" stroke-width="{1 if t == 0 else 0.8}"/>\n'
        body += f'        <text x="88" y="{y + 3}" fill="{s["muted"]}" font-size="8" font-family="{MONO}" text-anchor="end">{t} ms</text>\n'
    pitch, w = 164, 88
    for i, (j, v) in enumerate(rows):
        x = 136 + i * pitch
        focal = j == rows[-1][0]
        fill = s["accent_tint"] if focal else f"rgba({s['ink_rgb']},0.06)"
        stroke = s["accent"] if focal else s["series1"]
        top_y = ys(v)
        body += f'        <rect x="{x}" y="{top_y}" width="{w}" height="{y0 - top_y}" fill="{fill}" stroke="{stroke}" stroke-width="1"/>\n'
        body += (f'        <text x="{x + w // 2}" y="{top_y - 8}" fill="{s["accent"] if focal else s["muted"]}" font-size="9" '
                 f'font-family="{MONO}" text-anchor="middle">{v:.0f} ms</text>\n')
        label = f"{j} thread" + ("" if j == 1 else "s")
        body += (f'        <text x="{x + w // 2}" y="{y0 + 20}" fill="{s["ink"]}" font-size="11" font-weight="600" '
                 f'font-family="{SANS}" text-anchor="middle">{label}</text>\n')
    n = DATA["corpus"]["files"]
    body += (f'        <text x="508" y="404" fill="{s["muted"]}" font-size="8" font-family="{MONO}" text-anchor="middle" '
             f'letter-spacing="0.04em">origami --check over {n:,} files, median of 5 runs</text>\n')
    title = "Checking the whole corpus"
    desc = ("Bar chart of the time origami needs to check the whole test corpus with 1, 2, 4, 8 and 10 "
            "threads: " + ", ".join(f"{v:.0f} ms" for _, v in rows) + ".")
    caption = (f"{DATA['machine']['cpu']} ({DATA['machine']['cores']} cores), {DATA['date']}. "
               "By default origami uses one thread per core.")
    return page(slug, title, desc, 960, 420, body, s, caption)


if __name__ == "__main__":
    for theme in SKINS:
        suffix = "" if theme == "light" else "-dark"
        (HERE / f"pipeline{suffix}.html").write_text(pipeline(theme))
        (HERE / f"benchmark{suffix}.html").write_text(benchmark(theme))
        (HERE / f"granularity{suffix}.html").write_text(granularity(theme))
        (HERE / f"latency{suffix}.html").write_text(latency(theme))
        (HERE / f"threads{suffix}.html").write_text(scaling(theme))
    print("written:", sorted(p.name for p in HERE.glob("*.html")))
