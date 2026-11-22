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

SKINS = {
    "light": dict(paper="#f4f6f9", sheet="#ffffff", ink="#18214d", muted="#4a5478",
                  soft="#5b6589", crease="#c9d1de", accent="#2456d6",
                  accent_tint="rgba(36,86,214,0.08)", ink_rgb="24,33,77",
                  series1="#6e7899"),
    "dark": dict(paper="#0e1430", sheet="#151d42", ink="#e7ebf6", muted="#aeb6d3",
                 soft="#8e97ba", crease="#2e3866", accent="#7fa2ff",
                 accent_tint="rgba(127,162,255,0.10)", ink_rgb="231,235,246",
                 series1="#7d86a8"),
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
               "comment with your file. Only whitespace, line breaks and keyword case may differ.")
    body = f'        <g transform="translate(0,-100)">\n{body}        </g>\n'
    return page(slug, title, desc, 960, 380, body, s, caption)


def benchmark(theme):
    s = SKINS[theme]
    slug = "benchmark" if theme == "light" else "benchmark-dark"
    acb = DATA["abap_cleaner"]
    threads = {r["threads"]: r["time"]["p50"] for r in DATA["corpus_by_threads"]}
    one, ten = threads[1] / 1000, threads[10] / 1000
    cleaner = acb["corpus_warm_ms"] / 1000
    rows = [("ABAP Cleaner, default profile", cleaner, False),
            ("origami, one thread", one, True),
            ("origami, ten threads", ten, False)]
    x0, x1 = 264, 904
    ceil = math.ceil(cleaner / 2) * 2
    px = (x1 - x0) / ceil
    body = ""
    for t in range(0, ceil + 1, 2):
        x = round(x0 + t * px)
        stroke = f"rgba({s['ink_rgb']},0.25)" if t == 0 else f"rgba({s['ink_rgb']},0.08)"
        body += f'        <line x1="{x}" y1="40" x2="{x}" y2="232" stroke="{stroke}" stroke-width="{1 if t == 0 else 0.8}"/>\n'
        body += f'        <text x="{x}" y="252" fill="{s["muted"]}" font-size="8" font-family="{MONO}" text-anchor="middle">{t} s</text>\n'
    for i, (name, v, focal) in enumerate(rows):
        y = 56 + i * 60
        w = max(2, round(v * px))
        fill = s["accent_tint"] if focal else f"rgba({s['ink_rgb']},0.06)"
        stroke = s["accent"] if focal else s["series1"]
        body += f'        <rect x="{x0}" y="{y}" width="{w}" height="32" fill="{s["paper"]}"/>\n'
        body += f'        <rect x="{x0}" y="{y}" width="{w}" height="32" fill="{fill}" stroke="{stroke}" stroke-width="1"/>\n'
        body += (f'        <text x="{x0 - 12}" y="{y + 20}" fill="{s["ink"]}" font-size="12" font-weight="600"'
                 f' font-family="{SANS}" text-anchor="end">{name}</text>\n')
        val = f"{v:.2f} s" + (f", {cleaner / v:.0f}× faster" if focal else "")
        body += (f'        <text x="{x0 + w + 10}" y="{y + 20}" fill="{s["accent"] if focal else s["muted"]}"'
                 f' font-size="9" font-family="{MONO}">{val}</text>\n')
    n_abap = DATA["corpus"]["abap_files"]
    n_all = DATA["corpus"]["files"]
    body += (f'        <text x="584" y="276" fill="{s["muted"]}" font-size="8" font-family="{MONO}" text-anchor="middle"'
             f' letter-spacing="0.04em">time for the test corpus: ABAP Cleaner {n_abap:,} ABAP files, origami all {n_all:,} files</text>\n')
    title = "Formatting the whole test corpus"
    desc = (f"Bar chart of the time to process the test corpus: ABAP Cleaner takes {cleaner:.2f} s with its "
            f"default profile; origami takes {one:.2f} s on one thread and {ten:.2f} s on ten threads.")
    caption = (f"{DATA['machine']['cpu']}, {DATA['date']}. ABAP Cleaner also changes code (about 100 rules); "
               "origami only changes layout. Method and all numbers: docs/benchmark.md.")
    return page(slug, title, desc, 960, 292, body, s, caption)


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
        (HERE / f"latency{suffix}.html").write_text(latency(theme))
        (HERE / f"threads{suffix}.html").write_text(scaling(theme))
    print("written:", sorted(p.name for p in HERE.glob("*.html")))
