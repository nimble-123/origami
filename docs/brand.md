# origami: visual design

The visual language for the website, the README, the diagrams and the VS Code
extension. Everything here serves one idea:

> **Every line break is a fold.** origami takes a long line of ABAP and folds
> it into shape, and it can say why it folded where it did.

The motif comes from origami crease patterns, the line drawings that show
where a sheet is folded. By convention they draw **mountain folds in red** and
**valley folds in blue**. origami uses the same vocabulary: blue marks what
origami adds (a valley fold, a new line, a matched token), red marks what goes
wrong or away (a token that differs, an error). Creases are dashed lines.

The second idea is the editor's column. ABAP developers count line width in
columns, so the page sets its headlines in a monospaced face: every letter is
one column, and a headline folds at a column the way a statement does.

The colors come from the [Tokyo Night](https://github.com/tokyo-night/tokyo-night-vscode-theme)
themes for VS Code: **Tokyo Night Light** by day, **Tokyo Night** by night.
Many ABAP developers already read code in these colors, so the page, the
diagrams and the terminal look like their editor.

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
| `--sheet` | `#E6E7ED` | `#1A1B26` | sheets: code, the journey's stage, the playground |
| `--ink` | `#343B58` | `#C0CAF5` | text and lines |
| `--ink-soft` | `#565A6E` | `#A9B1D6` | secondary text |
| `--ink-dim` | `#6C6E75` | `#737AA2` | line numbers, steps not in view, disabled |
| `--valley` | `#34548A` | `#7AA2F7` | valley fold: added lines, matched tokens, links, the margin, primary action |
| `--mountain` | `#8C4351` | `#F7768E` | mountain fold: a token that differs, errors; used sparingly |
| `--crease` | `#A8AAB6` | `#3B4261` | crease lines, borders, dividers |
| `--valley-wash` | `#D1D5E1` | `#20303B` | hover and selected backgrounds, added-line background |
| `--mountain-wash` | `#DBD3DA` | `#37222C` | background of a token that differs |

Syntax colors in code, from the same themes:

| Token | Light | Dark | Use |
|---|---|---|---|
| `--syn-keyword` | `#5A4A78` | `#BB9AF7` | ABAP keywords |
| `--syn-string` | `#485E30` | `#9ECE6A` | string literals and templates; also marks what origami copies as it is |
| `--syn-number` | `#965027` | `#FF9E64` | numbers |
| `--syn-comment` | `#5F6272` | `#828BB8` | comments, italic |
| `--syn-rule` | `#0F4B6E` | `#7DCFFF` | rule IDs in notes (`ABAP-CALL-001`) |

The page and sheet values, `--ink`, the accents and the syntax colors are the
theme's own. `--sheet` (light), `--crease` (light) and the light washes are
mixed from them. Comments are lighter in the editor themes (`#9699A3`,
`#565F89`); here they are raised to reach 4.5:1.

Rules:

* Blue and red carry meaning. They are never decoration and never a gradient.
* Text meets WCAG AA on `--paper` and `--sheet`: `--ink` 7.6:1 (light) and
  11.1:1 (dark), `--valley` 5.2:1 and 7.1:1, every syntax color at least
  4.5:1 on `--sheet`. `--ink-dim` (about 4:1) is for line numbers and labels
  that repeat something said elsewhere.
* Syntax colors appear only in code. Keywords are colored, not bold.

## Type

Two families, both variable, self-hosted as Latin WOFF2 (SIL OFL 1.1, see
`THIRD_PARTY_NOTICES.md`):

* **Martian Mono** (width 75–112.5, weight 100–800): headlines, the brand
  name in the header, code, numbers in tables, notes. Headlines are set
  condensed (width 75) and heavy (700–780), tight (−0.02em); code at width
  100, weight 400. One letter is one column, so the hero's headline folds at
  a column, like a statement, and code and headlines share one grid.
* **Archivo** (weight 400–800): everything that is read: text, labels,
  buttons. A plain grotesque with a slightly technical cut; it stays quiet
  next to the monospaced headlines.

Why these: in the browser, Schibsted Grotesk with Martian Mono (the earlier
pair), Archivo expanded and condensed with JetBrains Mono, Bricolage
Grotesque and Hanken Grotesk were set side by side with the hero and a code
sample. A proportional headline reads as any product page; the monospaced,
condensed headline is the only one that says "columns". JetBrains Mono turns
`=>` into an arrow by default, which misstates ABAP. Martian Mono is kept for
code; its width axis is new here and does the headlines.

Scale (ratio 1.333, from 17px): 12.75 · 17 · 22.7 · 30.2 · 40.3 · 53.7 ·
71.6. The hero headline is sized to its column (20 columns on wide screens,
14 on phones). Body text 17px / 1.6; the lede 22.7px. Sentence case
everywhere; no all-caps labels, no small monospaced labels on non-code
content. Prose at most 62 characters wide. Numbers that change use tabular
figures.

## Layout

* **Grid:** 12 columns, 1.5rem gaps, at most 76rem wide, side margins
  `clamp(1rem, 4vw, 2.5rem)`.
* **Space:** steps of 4px: 0.25, 0.5, 0.75, 1, 1.5, 2, 3, 4, 6, 8rem.
  Sections are 4–7.5rem apart and separated by a crease.
* **Order:** hero; the journey (five steps); what happens when the check
  fails; the playground; speed and install side by side (stacked on
  phones); the closing photograph; footer.

```
1440                                          390
+--------------------------------------+     +----------------------+
| origami     How it works ... GitHub  |     | origami        Menu  |
+--------------------------------------+     +----------------------+
| A long line of ABAP,      [crane]    |     |      [crane]         |
| folded into shape.                   |     | A long line of       |
| lede          $ brew install ...     |     | ABAP, folded         |
+-------------------+------------------+     | into shape.          |
| STAGE (pinned)    | 1 Read           |     | lede, install        |
|  sheet: the call  |                  |     +----------------------+
|  margin  | 72     | 2 Fold           |     | STAGE (pinned, 46%)  |
|  foot: counter    |                  |     |  sheet, margin, foot |
|  [crane]          | 3 Check          |     |  3 Check   [crane]   |
|                   | 4 Explain        |     +----------------------+
|                   | 5 Write          |     | the steps scroll     |
+-------------------+------------------+     | beneath the stage    |
| When the check fails (pipeline)      |     +----------------------+
| Playground (full width)              |     | ...                  |
| Speed (5 cols)  | Install (6 cols)   |
| closing photograph                   |
+--------------------------------------+
```

## Shape and line

* **Crease line:** a 1px dashed line in `--crease`, the divider between
  sections and steps. A dashed `--valley` line is a fold origami makes: the
  margin, the creases in the hero headline, the creases on the paper crane.
* **Folded corner:** the one decorative device, a 22px triangle on the
  journey's sheet. Once per view.
* **Corners:** 4px on sheets and buttons; nothing is pill-shaped except the
  start and end nodes of the pipeline diagram.
* **No card grids.** Content sits on the paper in columns; only code, the
  stage and the playground sit on sheets.

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

## Imagery

One subject: a real-looking paper crane, folded like the logo, in the three
logo tones of the current theme, on a plain ground of the theme's paper.
Soft light from the left, one shadow, the crane right of center so the left
part stays empty for a headline. No text, props, hands or people.

| File (`docs/assets/`) | Use |
| --- | --- |
| `photos/fold-light.webp`, `photos/fold-dark.webp` | the closing photograph of the website ("Every line break is a fold"), light and dark, 1920×1072, 36 and 47 KB |
| `social-preview.png` | the link preview, 1280×640: `fold-dark` with the headline, rendered from `site/og/og.html` with a headless browser |
| `paper-fibre.webp` | 512px greyscale paper grain, the 3D crane's bump map and a faint grain in its colors; never on flat surfaces |
| `photos/crane-light.webp`, `photos/crane-dark.webp`, `photos/crane-detail.webp` | the earlier photographs; no longer on the website |

The photographs and the paper grain are generated with Nano Banana Pro
(Gemini 3 Pro Image): the photographs from `logo.svg` as the reference and a
prompt that names the hex values of the facets and the ground; the grain from
a text prompt, then scaled, turned grey and raised in contrast. Keep the
source files out of the repository; commit WebP at 75–86 quality, under
80 KB each.

## 3D crane

The website draws the logo as a crane of real paper in WebGL
(`site/crane.js`, three.js r170 in `site/vendor/`).

* **Geometry:** the seven facets of `logo.svg`, each a thin, rigid sheet
  (about 1% of the crane's width thick) with the logo's gaps between them.
  Every facet hangs from a neighbour and turns about the edge they share: the
  halves of the body about the middle, the wings, the neck and the tail about
  the body, the head about the neck. Facets never bend.
* **Fold:** one number from 0 to 1. At 0 it is a flat sheet of paper on the
  table, seen from above at an angle: a sheet behind the facets closes the
  gaps, the facets take the paper's color, and dashed `--valley` creases
  mark the hinges;
  as it folds, it stands up and the facets turn in stages: body, wings, neck
  and tail, head. At 1 the crane stands.
* **Materials and light:** matte paper in `--ink`, `--valley` and `--crease`,
  read from the page's tokens, so it changes with the color scheme. A soft
  key light from the upper left, a light close to the crane so the light
  falls off across each facet, a sky light, and one soft shadow on the
  ground. No glow, bloom, particles, gradients or glassy materials.
* **Where:** one canvas for the page. It stands in the hero, then travels
  with the scroll to the journey's stage, lies down flat for Read and folds
  through Fold and Check to standing at Explain. In the hero it turns a
  little toward the mouse, and a sideways drag turns it within limits and
  lets it ease back; vertical swipes scroll the page.
* **Limits:** loaded after the first paint; drawn only while something
  moves (no idle frames), while it is on screen and the tab is visible; at
  most 2 device pixels per CSS pixel. The canvas is decorative
  (`aria-hidden`); its place is reserved, so nothing moves when it appears.
  Without WebGL 2, with Save-Data, on devices that report less than 2 GB of
  memory, or when the module fails to load, the page shows `logo.svg` /
  `logo-dark.svg` instead, smaller than the headline.

## The journey

The heart of the website: one overlong statement (104 characters) through
what origami does with it, in five numbered steps (the only numbered markers
on the page, because it is a sequence). A stage stays pinned while the
steps scroll past: on wide screens beside the steps, its centre on the line
where the active step's text sits, with the sheet (code at 15px, as tall as
its tallest state), the crane on the paper ground at its lower right, and a
rail of the five steps (links, the current one marked); on phones at the top
(half the screen), with a band of paper beneath it and the rail as numbers.
A step takes about 85% of the screen height on wide screens and 62% on
phones. Where the sheet cuts the long line, a tab says how many columns are
out of view. Every state on the sheet is computed by origami in the page's
worker; nothing is drawn from a script of the result.

1. **Read:** the tokens from origami's lexer (`lex` in `origami_wasm`) get
   their boxes one by one; a literal is marked as copied as it is.
2. **Fold:** the line folds into the layout `review` returns for the width;
   the margin can be dragged or moved with the keys (40 to 120); the sheet
   uses the style `compact`, and says so.
3. **Check:** the output is lexed again. Above the code, the line as
   written; a cursor walks the tokens with the scroll, marks each token in
   both rows at once with one line between the pair, leaves a quiet
   `--valley` underline behind, and the counter counts up to "20 of 20 tokens
   identical". "What if a token changed?" removes one space in the output, in
   the page only, labelled as a simulation: the cursor stops at the token
   that differs, the pair turns `--mountain`, the statement goes back to how
   it was written, and W001 is quoted as `check.rs` reports it.
4. **Explain:** the `--review` note under the construct, with its rule linked
   to STYLE.md.
5. **Write:** how long the run took in this browser.

## Motion

* **One load moment:** the hero headline stands as one long line running off
  the right edge, dashed creases appear at its break points, and it folds
  into its lines (about 1.8 seconds, CSS only). The crane in the hero folds
  up from its sheet with it.
* **Scroll-linked:** the journey's stage follows the scroll position (a
  scroll listener throttled to one update per frame, never a change to
  native scrolling, no snapping), and the crane travels from the hero to the
  stage. No fade-and-slide entrances anywhere.
* **Answers to actions:** 150–220ms, `cubic-bezier(0.2, 0, 0, 1)`,
  interruptible transitions on named properties; buttons press to 0.96.
* **Reduced motion:** no load moment (the headline is shown folded), no
  travel; the journey is a plain sequence of steps with static figures, the
  same as without scripting.

## Voice

Plain, specific, calm. Say what origami does to the code ("puts each
parameter on its own line when the call is too long"), not how good it is.
Numbers are measured and linked to their source; the page shows the
published ones and measures nothing it does not say it measured. No
exclamation marks.

## Where it applies

* **Website (`site/`):** `index.html` (hero, journey, check pipeline,
  playground, speed, install, closing photograph) and `abap-cleaner.html`
  (generated by `scripts/abap-cleaner-compare/build.py`), sharing the header,
  `style.css` and `nav.js`. The no-script figures of the journey are the only
  formatted output written into `index.html`; regenerate them when the style
  changes.
* **README:** logo, one sentence, badges, the animated terminal (light and
  dark), diagrams in light and dark SVG.
* **Diagrams:** the benchmark charts and `pipeline*.svg` in the palette above
  (light and dark SVG), crease-line frames, Martian Mono for code. On the
  website the pipeline is drawn in the page as SVG with the page's tokens.
