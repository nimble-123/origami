#!/bin/sh
# Builds the website into site/dist: the static page, the diagrams and
# origami compiled to WebAssembly. Serve it with any static server, e.g.
#   sh site/build.sh && python3 -m http.server -d site/dist 8000
set -eu
cd "$(dirname "$0")/.."
rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true
cargo build --release -p origami_wasm --target wasm32-unknown-unknown
rm -rf site/dist
mkdir -p site/dist/assets
cp site/index.html site/style.css site/app.js site/dist/
cp target/wasm32-unknown-unknown/release/origami_wasm.wasm site/dist/origami.wasm
for f in logo logo-dark pipeline pipeline-dark benchmark benchmark-dark review-demo review-demo-dark; do
  cp "docs/assets/$f.svg" site/dist/assets/
done
echo "site/dist: $(du -sh site/dist | cut -f1)"
