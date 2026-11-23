# Diagrams

Sources of the diagrams in `docs/assets/` (README and website). Colors and
type follow [docs/brand.md](../brand.md).

```sh
python3 docs/diagrams/build.py          # writes pipeline, benchmark, granularity, latency, threads (light and dark)
# export the <svg> of each page to docs/assets/<name>.svg, e.g. with the
# diagram-design skill: scripts/export_svg.py docs/diagrams/pipeline.html docs/assets/pipeline.svg
python3 docs/demo/terminal.py           # docs/assets/review-demo*.svg (needs target/release/origami)
```

`latency` and `threads` read [docs/benchmark-data.json](../benchmark-data.json),
which `cargo xtask bench-matrix` writes together with
[docs/benchmark.md](../benchmark.md). `benchmark` (the whole project) and
`granularity` (one statement, one method, one file, cold and warm) read
[docs/benchmark-compare-data.json](../benchmark-compare-data.json), which
`cargo xtask bench-compare` writes together with
[docs/benchmark-abap-cleaner.md](../benchmark-abap-cleaner.md). After a new
measurement, run `build.py` and export the SVGs again.
