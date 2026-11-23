#!/bin/sh
# Builds the website into site/dist: the static page, the diagrams and
# origami compiled to WebAssembly. Serve it with any static server, e.g.
#   sh site/build.sh && python3 -m http.server -d site/dist 8000
# The link preview image (docs/assets/social-preview.png) is rendered from
# site/og/og.html with a headless browser at 1280x640.
set -eu
cd "$(dirname "$0")/.."
rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true
cargo build --release -p origami_wasm --target wasm32-unknown-unknown
rm -rf site/dist
mkdir -p site/dist/assets
cp site/index.html site/abap-cleaner.html site/style.css site/app.js site/worker.js site/dist/
cp target/wasm32-unknown-unknown/release/origami_wasm.wasm site/dist/origami.wasm
# Keywords for the playground's highlighting: the words origami's parsers know.
abap=$(grep -rohE '"[A-Z][A-Z-]*[A-Z]"' crates/origami_abap/src | tr -d '"' | sort -u | tr '\n' ' ')
cds=$(grep -rohE '"[a-z][a-z_]*[a-z]"' crates/origami_cds/src | tr -d '"' | sort -u | tr '\n' ' ')
printf 'self.KEYWORDS = { abap: "%s", cds: "%s" };\n' "$abap" "$cds" > site/dist/keywords.js
for f in logo logo-dark pipeline pipeline-dark benchmark benchmark-dark latency latency-dark review-demo review-demo-dark; do
  cp "docs/assets/$f.svg" site/dist/assets/
done
cp docs/assets/social-preview.png site/dist/assets/
echo "site/dist: $(du -sh site/dist | cut -f1)"
