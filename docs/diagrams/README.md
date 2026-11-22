# Diagrams

Sources of the diagrams in `docs/assets/` (README and website). Colors and
type follow [docs/brand.md](../brand.md).

```sh
python3 docs/diagrams/build.py          # writes pipeline*.html, benchmark*.html
# export the <svg> of each page to docs/assets/<name>.svg, e.g. with the
# diagram-design skill: scripts/export_svg.py docs/diagrams/pipeline.html docs/assets/pipeline.svg
python3 docs/demo/terminal.py           # docs/assets/review-demo*.svg (needs target/release/origami)
```

The benchmark numbers come from [docs/benchmark-abap-cleaner.md](../benchmark-abap-cleaner.md);
update both together.
