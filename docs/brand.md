# origami: visual design

The visual language for the README, the website, the diagrams and the VS Code
extension. Everything here serves one idea:

> **Every line break is a fold.** origami takes a long line of ABAP and folds
> it into shape, and it can say why it folded where it did.

The motif comes from origami crease patterns, the line drawings that show
where a sheet is folded. By convention they draw **mountain folds in red** and
**valley folds in blue**, on plain paper. origami uses the same vocabulary:
blue marks what origami adds (a valley fold, a new line), red marks what goes
away. The paper is cool and plain, the ink is indigo, and creases are drawn as
dashed lines.

## Audience and job

* **Who:** ABAP developers and SAP teams; many work in ADT for Eclipse or
  VS Code, review in abapGit, and are wary of tools that touch their code.
* **What the page has to do:** show in five seconds what origami does to real
  ABAP, prove that it never changes the meaning of the code, and get it
  installed (`brew install`, VS Code, CI).

## Color

Base palette (light):

| Token | Hex | Use |
|---|---|---|
| `--paper` | `#F4F6F9` | page background: cool, untinted paper |
| `--sheet` | `#FFFFFF` | the "sheet": code panels, the playground |
| `--ink` | `#18214D` | text and lines: indigo ink, never pure black |
| `--valley` | `#2456D6` | valley fold: added lines, links, primary action |
| `--mountain` | `#D23A49` | mountain fold: removed lines, errors; used sparingly |
| `--crease` | `#C9D1DE` | crease lines, borders, dividers |

Supporting tokens: `--ink-soft #4A5478` (secondary text), `--valley-wash
#E6EDFC` (added-line background), `--mountain-wash #FBE9EB` (removed-line
background), `--fold-shadow rgba(24, 33, 77, 0.10)` (the only shadow, under a
folded edge).

Dark mode is the same sheet at night, not an inverted page:

| Token | Hex |
|---|---|
| `--paper` | `#0E1430` |
| `--sheet` | `#151D42` |
| `--ink` | `#E7EBF6` |
| `--valley` | `#7FA2FF` |
| `--mountain` | `#FF7A86` |
| `--crease` | `#2E3866` |

Rules:

* Blue and red carry meaning (added / removed, fold / unfold). They are never
  decoration and never a gradient.
* Text on `--paper` and `--sheet` meets WCAG AA; `--valley` on `--paper` is
  5.7:1 (light) and 7.3:1 (dark). On the diff washes, text stays `--ink`;
  red and blue mark only the `-` / `+` signs and the changed characters
  (red on its wash is 4.0:1, too low for running text).
* Syntax highlighting in code stays quiet: keywords in `--ink` bold, strings
  and comments in `--ink-soft`; only the diff uses color.

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

A square sheet folded along its diagonal. The upper-left half carries three
lines of code that get shorter towards the crease; the lower-right half is
the folded flap, showing the back of the paper in `--crease`; between them
runs the crease as a dashed `--valley` line. Outline and code lines in
`--ink`. Wordmark: "origami" in Schibsted Grotesk 800, lower case. Files:
`docs/assets/logo.svg`, `docs/assets/logo-dark.svg`.

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
* **Diagrams:** pipeline and benchmark in the palette above, crease-line
  frames, Schibsted Grotesk labels, Martian Mono for code.
