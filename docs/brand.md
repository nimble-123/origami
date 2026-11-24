# origami: visual design

The visual language for the README, the website, the diagrams and the VS Code
extension. Everything here serves one idea:

> **Every line break is a fold.** origami takes a long line of ABAP and folds
> it into shape, and it can say why it folded where it did.

The motif comes from origami crease patterns, the line drawings that show
where a sheet is folded. By convention they draw **mountain folds in red** and
**valley folds in blue**. origami uses the same vocabulary: blue marks what
origami adds (a valley fold, a new line), red marks what goes away. Creases
are drawn as dashed lines.

The colors come from the [Tokyo Night](https://github.com/tokyo-night/tokyo-night-vscode-theme)
themes for VS Code: **Tokyo Night Light** by day, **Tokyo Night** by night.
Many ABAP developers already read code in these colors, so the page, the
diagrams and the terminal look like their editor. Tokyo Night's blue and red
are the valley and mountain folds.

## Audience and job

* **Who:** ABAP developers and SAP teams; many work in ADT for Eclipse or
  VS Code, review in abapGit, and are wary of tools that touch their code.
* **What the page has to do:** show in five seconds what origami does to real
  ABAP, prove that it never changes the meaning of the code, and get it
  installed (`brew install`, VS Code, CI).

## Color

Two themes, chosen by the reader's system setting (`prefers-color-scheme`).
Every token has a value in both.

| Token | Light (Tokyo Night Light) | Dark (Tokyo Night) | Use |
|---|---|---|---|
| `--paper` | `#D5D6DB` | `#16161E` | page background |
| `--sheet` | `#E6E7ED` | `#1A1B26` | the "sheet": code panels, the playground |
| `--ink` | `#343B58` | `#C0CAF5` | text and lines |
| `--ink-soft` | `#565A6E` | `#A9B1D6` | secondary text |
| `--ink-dim` | `#6C6E75` | `#737AA2` | line numbers only |
| `--valley` | `#34548A` | `#7AA2F7` | valley fold: added lines, links, primary action |
| `--mountain` | `#8C4351` | `#F7768E` | mountain fold: removed lines, errors; used sparingly |
| `--crease` | `#A8AAB6` | `#3B4261` | crease lines, borders, dividers |
| `--valley-wash` | `#D1D5E1` | `#20303B` | added-line background |
| `--mountain-wash` | `#DBD3DA` | `#37222C` | removed-line background |

Syntax colors in code, from the same themes:

| Token | Light | Dark | Use |
|---|---|---|---|
| `--syn-keyword` | `#5A4A78` | `#BB9AF7` | ABAP keywords |
| `--syn-string` | `#485E30` | `#9ECE6A` | string literals and templates |
| `--syn-number` | `#965027` | `#FF9E64` | numbers |
| `--syn-comment` | `#5F6272` | `#828BB8` | comments, italic |
| `--syn-rule` | `#0F4B6E` | `#7DCFFF` | rule IDs in notes (`ABAP-CALL-001`) |

The page and sheet values, `--ink`, the accents and the syntax colors are the
theme's own. `--sheet` (light), `--crease` (light) and the light washes are
mixed from them. Comments are lighter in the editor themes (`#9699A3`,
`#565F89`); here they are raised to reach 4.5:1.

Rules:

* Blue and red carry meaning (added / removed, fold / unfold). They are never
  decoration and never a gradient.
* Text meets WCAG AA on `--paper` and `--sheet`: `--ink` 7.6:1 (light) and
  11.1:1 (dark), `--valley` 5.2:1 and 7.1:1, every syntax color at least
  4.5:1 on `--sheet`. `--ink-dim` (about 4:1) is for line numbers only.
* On the diff washes, text stays `--ink`; red and blue mark the `-` / `+`
  signs and the changed characters.
* Syntax colors appear only in code. Keywords are colored, not bold.

## Type

* **Schibsted Grotesk** (Google Fonts, 400/500/700/800) for headlines and
  text. A sturdy grotesk with a newspaper background: plain, readable,
  slightly condensed at heavy weights. Headlines at 800 with tight tracking
  (−0.02em); body at 400, 17px / 1.55.
* **Martian Mono** (Google Fonts, 400/600) for all code, commands and the
  terminal. Wide and square, so ABAP keywords read as blocks, the way an
  ABAP editor shows them.

Scale (ratio 1.25, from 17px): 13.6 · 17 · 21.3 · 26.6 · 33.2 · 41.5 · 51.9.
Headlines use sentence case. No all-caps labels, no small monospace labels on
non-code content. Line length: at most 68 characters for prose.

## Shape and line

* **Crease line:** a 1px dashed line in `--crease` (dash 6, gap 4), the
  divider between page sections and the frame of diagrams. A blue dashed line
  (`--valley`) marks a fold that origami adds.
* **Folded corner:** the one decorative device. A sheet (code panel) may have
  its top-right corner folded down (a 20px triangle in `--crease` with
  `--fold-shadow`). Used once per view, on the main sheet.
* **Corners:** sheets 4px radius (paper is almost square); buttons 4px;
  nothing is pill-shaped.
* **No cards grid.** Content sits on the paper in columns; only code and the
  playground sit on sheets.

## Logo

A paper crane as a low-poly model: seven flat facets, each one a fold. Every
edge runs at a multiple of 15°, so the crane looks constructed, not drawn. The
facets take three roles: `--ink`, `--valley` and `--crease`, each in its
theme's value. Between the facets runs a gap (the folds): 5 units on the
256-unit canvas in the light files, 6 units in the dark and white files,
because light shapes on a dark ground look larger and would close the gaps.
The head joins the neck without a gap. The outline has no stroke. Wordmark:
"origami" in Schibsted Grotesk 800, lower case, tracking −0.02em, set as paths
(SIL Open Font License, which allows logo use).

### Files

All in `docs/assets/`. Light files go on light grounds, `-dark` files on dark
grounds.

| File | Use |
| --- | --- |
| `logo.svg`, `logo-dark.svg` | the mark, 48px and up |
| `logo-small.svg`, `logo-small-dark.svg` | 32px and below: no gaps, larger in the canvas (site header) |
| `logo-lockup.svg`, `logo-lockup-dark.svg` | mark and wordmark side by side (README header) |
| `logo-lockup-stacked.svg`, `logo-lockup-stacked-dark.svg` | mark over the wordmark, for square spaces |
| `logo-wordmark.svg`, `logo-wordmark-dark.svg` | the wordmark alone, where the mark is already on screen |
| `logo-mono.svg`, `logo-white.svg` | one color: `--ink` (light) and white, e.g. on `--valley` |
| `logo-tile.svg` | the dark mark on a `--sheet` (dark) tile, filling 85% of its width: VS Code Marketplace icon, app icon |
| `favicon.svg` | the small cut; switches to the dark tones with the browser |

The PNG icons (`editors/vscode/icon.png` at 256px,
`docs/assets/apple-touch-icon.png` at 180px, without rounded corners because
iOS rounds them) are rendered from the tile. The social preview
(`docs/assets/social-preview.png`, from `site/og/og.html`) uses the dark theme.

### Colors

| Facet | Light (Tokyo Night Light) | Dark (Tokyo Night) |
| --- | --- | --- |
| neck, body right | `#343B58` · RGB 52 59 88 | `#C0CAF5` · RGB 192 202 245 |
| head, body left, wing back | `#34548A` · RGB 52 84 138 | `#7AA2F7` · RGB 122 162 247 |
| wing front, tail | `#A8AAB6` · RGB 168 170 182 | `#3B4261` · RGB 59 66 97 |
| tile | – | `#1A1B26` |

The logo is made for screens. For print, use `logo-mono.svg` and a CMYK
proof; a straight conversion of `#343B58` is C41 M33 Y0 K65.

### Rules

* **Clear space:** keep a quarter of the mark's height free on every side
  (for lockups: a quarter of the lockup's height).
* **Minimum size:** `logo.svg` 48px wide; `logo-small.svg` 16px; horizontal
  lockup 24px high.
* **Lockup spacing:** the gap between mark and wordmark is a third of the
  lockup's height; do not set the wordmark in live text next to the mark.
* **Backgrounds:** light files on `--paper`, `--sheet` or white; dark files on
  `--paper`/`--sheet` dark or GitHub's dark background; `logo-white.svg` on
  `--valley` or photos. The light files on `--valley` lose their blue facets.
* **Don't:** recolor single facets, add outlines, shadows or gradients, rotate
  or mirror the crane, close the gaps in the large files, or put the dark
  files on a light ground (the `--crease` facets disappear).

### In use

![The logo on a GitHub README, as an app icon, in a website header, next to a terminal, as a sticker and as a social profile](assets/logo-in-use.webp)

## Photography

One subject: a real-looking paper crane, folded like the logo, in the three
logo tones of the current theme, on a plain ground of the theme's paper.
Soft light from the left, one shadow, the crane right of center so the left
third stays empty for a headline. No text, props or hands in the picture.

| File | Use |
| --- | --- |
| `photos/crane-light.webp`, `photos/crane-dark.webp` | the photo band on the website (light and dark), 1920px |
| `photos/crane-detail.webp` | a square close-up, next to "Every fold has a reason" on the website |
| `social-preview.png` | `crane-dark` with the headline, from `site/og/og.html` |

The photos are generated with Nano Banana Pro (Gemini 3 Pro Image) from
`logo.svg` as the reference and a prompt that names the hex values of the
facets and the ground. Keep the source PNGs out of the repository; commit
WebP at about 80 quality (under 50 KB each).

## Motion

One orchestrated moment: the hero. A single overlong ABAP statement appears
on the sheet; dashed blue creases appear at the break points (staggered, 80ms
each); then the line folds down into its formatted shape and the notes
(`↳ ABAP-CALL-001: …`) fade in under it. About 1.6 seconds, once per load.
With `prefers-reduced-motion`, the formatted result is shown directly.

Motion that answers an action (the playground formatting, copying a command,
switching tabs) is short (150–200ms, ease-out) and shows what changed. No
scroll-triggered entrances.

## Voice

Plain, specific, calm. Say what origami does to the code ("puts each
parameter on its own line when the call is too long"), not how good it is.
Numbers are measured and linked to their source. No exclamation marks.

## Where it applies

* **README:** logo, one sentence, badges, the animated terminal (light and
  dark), diagrams in light and dark SVG.
* **Website (`site/`):** a single page on GitHub Pages: hero with the fold
  animation, the playground (origami in WebAssembly), how the self-check
  works, line-break notes, speed, install.
* **Diagrams:** pipeline and the benchmark charts in the palette above (light and dark
  SVG), crease-line frames, Schibsted Grotesk labels, Martian Mono for code.
