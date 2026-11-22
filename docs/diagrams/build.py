#!/usr/bin/env python3
"""Builds the origami diagrams (light and dark HTML) from one description.

Palette and type: docs/brand.md. Run: python3 docs/diagrams/build.py
Then export the SVGs for README and website (see docs/diagrams/README.md).
"""
from pathlib import Path

HERE = Path(__file__).parent

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
    rows = [("ABAP Cleaner, default profile", 10.44, False),
            ("ABAP Cleaner, essential profile", 6.69, False),
            ("ABAP Cleaner, parse only", 1.93, False),
            ("origami, one thread", 0.70, True)]
    x0, x1, ceil = 264, 904, 12
    px = (x1 - x0) / ceil
    body = ""
    for t in range(0, ceil + 1, 2):
        x = round(x0 + t * px)
        stroke = f"rgba({s['ink_rgb']},0.25)" if t == 0 else f"rgba({s['ink_rgb']},0.08)"
        body += f'        <line x1="{x}" y1="40" x2="{x}" y2="292" stroke="{stroke}" stroke-width="{1 if t == 0 else 0.8}"/>\n'
        body += f'        <text x="{x}" y="312" fill="{s["muted"]}" font-size="8" font-family="{MONO}" text-anchor="middle">{t} s</text>\n'
    for i, (name, v, focal) in enumerate(rows):
        y = 56 + i * 60
        w = round(v * px)
        fill = s["accent_tint"] if focal else f"rgba({s['ink_rgb']},0.06)"
        stroke = s["accent"] if focal else s["series1"]
        body += f'        <rect x="{x0}" y="{y}" width="{w}" height="32" fill="{s["paper"]}"/>\n'
        body += f'        <rect x="{x0}" y="{y}" width="{w}" height="32" fill="{fill}" stroke="{stroke}" stroke-width="1"/>\n'
        body += (f'        <text x="{x0 - 12}" y="{y + 20}" fill="{s["ink"]}" font-size="12" font-weight="600"'
                 f' font-family="{SANS}" text-anchor="end">{name}</text>\n')
        val = f"{v:.2f} s" + (", 15× faster" if focal else "")
        body += (f'        <text x="{x0 + w + 10}" y="{y + 20}" fill="{s["accent"] if focal else s["muted"]}"'
                 f' font-size="9" font-family="{MONO}">{val}</text>\n')
    body += (f'        <text x="584" y="336" fill="{s["muted"]}" font-size="8" font-family="{MONO}" text-anchor="middle"'
             f' letter-spacing="0.04em">time for 1,652 ABAP files (426,831 lines), one thread, warm</text>\n')
    title = "Formatting the whole test corpus"
    desc = ("Bar chart of the time to process 1,652 ABAP files: ABAP Cleaner takes 10.44 s with its "
            "default profile, 6.69 s essential, 1.93 s parse only; origami takes 0.70 s on one thread.")
    caption = ("Apple M4, 2026-11-21. ABAP Cleaner also changes code (about 100 rules); origami only "
               "changes layout. With 10 threads origami takes 0.19 s. Method and numbers: "
               "docs/benchmark-abap-cleaner.md.")
    return page(slug, title, desc, 960, 352, body, s, caption)


if __name__ == "__main__":
    for theme in SKINS:
        suffix = "" if theme == "light" else "-dark"
        (HERE / f"pipeline{suffix}.html").write_text(pipeline(theme))
        (HERE / f"benchmark{suffix}.html").write_text(benchmark(theme))
    print("written:", sorted(p.name for p in HERE.glob("*.html")))
