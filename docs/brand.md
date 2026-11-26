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
| `photos/crane-light.webp`, `photos/crane-dark.webp` | the photo band near the end of the website, before Install (light and dark), 1920px |
| `photos/crane-detail.webp` | a square close-up, next to "Every fold has a reason" on the website |
| `social-preview.png` | `crane-dark` with the headline, from `site/og/og.html` |

The photos are generated with Nano Banana Pro (Gemini 3 Pro Image) from
`logo.svg` as the reference and a prompt that names the hex values of the
facets and the ground. Keep the source PNGs out of the repository; commit
WebP at about 80 quality (under 50 KB each).

### Paper texture

`paper-fibre.webp` (512px, greyscale, about 40 KB) is the grain of plain
matte paper: fine fibres, no folds, no light. The 3D crane uses it as a bump
map and as a faint grain in the facet colors, mirrored when it repeats, so
the tile shows no seam. Generated with Nano Banana Pro from a text prompt
(no reference image), then scaled, turned grey and raised in contrast. It is
not a decoration for flat surfaces: the sheets and the paper stay plain.

## 3D crane

The website's hero shows the logo as a crane of real paper (`site/crane.js`,
three.js r170 under `site/vendor/`).

* **Geometry:** the seven facets of `logo.svg`, each a thin, rigid sheet
  (about 1% of the crane's width thick) with the logo's gaps between them.
  Every facet hangs from a neighbour and turns about the edge they share: the
  two halves of the body about the middle, the wings, the neck and the tail
  about the body, the head about the neck. Facets never bend.
* **Fold:** a single number from 0 (flat: the logo laid out, with dashed
  `--valley` creases on the hinges) to 1 (folded). The facets fold in stages:
  body, wings, neck and tail, head. On the page the margin of the hero sheet
  sets it: one line is almost flat, more line breaks fold it further.
* **Materials:** matte paper (no gloss, no metal), in the logo's three roles
  `--ink`, `--valley` and `--crease`, read from the page's tokens, so the
  crane changes with the color scheme.
* **Light:** as in the photos: a soft key light from the upper left, a light
  close to the crane so the light falls off across each facet, a sky light,
  and one soft shadow on the paper ground. No glow, bloom, particles,
  gradients or glassy materials.
* **Motion:** a slow sway; it turns a little toward the pointer, and a
  sideways drag (mouse or one finger) turns it within limits and lets it ease
  back. Vertical swipes scroll the page. It stays on the ground.
* **Limits:** loaded after the first paint, rendered only while on screen and
  the tab is visible, at most 2 device pixels per CSS pixel. The canvas is
  decorative (`aria-hidden`); its box is reserved, so nothing moves when it
  appears. Without WebGL 2, with Save-Data, on devices that report less than
  2 GB of memory, or when the module fails to load, the page shows
  `logo.svg` / `logo-dark.svg` instead, smaller than the headline.

## Motion

One orchestrated moment: the hero, about 1.6 seconds, once per load. A
single overlong ABAP statement appears on the sheet with the margin at width
72 cutting through it; dashed blue creases appear at the break points
(staggered, 80ms each); the paper crane next to it shows its crease pattern;
then the line folds into its formatted shape (640ms, lines staggered 70ms)
while the crane folds up, stage by stage; the note (`↳ ABAP-CALL-001: …`)
fades in, and the margin nudges once to show that it can be dragged.

**The draggable margin.** The right margin of the hero sheet is a slider
(40 to 120; arrow keys ±1, Page Up/Down ±10, Home/End). Dragging it formats
the call again with origami itself (the WebAssembly build, at most once per
frame) and moves each token from its old place to its new one (220ms,
interruptible); the note says why the line broke at that width, and the
crane folds further for more line breaks and lies almost flat when the line
fits. Before origami.wasm is ready the margin is shown in grey and does not
move.

With `prefers-reduced-motion`, the formatted result and the folded crane are
shown directly; the margin still works, without transitions, and the crane
does not sway.

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
* **Website (`site/`):** a single page on GitHub Pages: the hero with the
  fold, the draggable margin and the 3D crane; the playground (origami in
  WebAssembly); how the self-check works; line-break notes; speed; the photo
  band as a closing image; install, as tabs. A sticky header on `--paper`
  that gets a crease once the page scrolls. The comparison page
  (`abap-cleaner.html`) shares the header and the styles.
* **Diagrams:** pipeline and the benchmark charts in the palette above (light and dark
  SVG), crease-line frames, Schibsted Grotesk labels, Martian Mono for code.
