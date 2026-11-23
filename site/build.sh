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
for f in logo logo-dark pipeline pipeline-dark benchmark benchmark-dark latency latency-dark review-demo review-demo-dark; do
  cp "docs/assets/$f.svg" site/dist/assets/
done
cp docs/assets/social-preview.png site/dist/assets/
echo "site/dist: $(du -sh site/dist | cut -f1)"
