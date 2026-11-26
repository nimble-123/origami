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
cp site/index.html site/abap-cleaner.html site/style.css site/app.js site/worker.js site/nav.js site/crane.js site/dist/
# three.js (MIT, see THIRD_PARTY_NOTICES.md) for the crane in the hero, loaded
# by crane.js only after the page has rendered.
mkdir -p site/dist/vendor
cp site/vendor/three.module.min.js site/vendor/three.LICENSE site/dist/vendor/
mkdir -p site/dist/fonts
cp site/fonts/*.woff2 site/dist/fonts/
cp target/wasm32-unknown-unknown/release/origami_wasm.wasm site/dist/origami.wasm
# Keywords for the playground's highlighting: the words origami's parsers know.
abap=$(grep -rohE '"[A-Z][A-Z-]*[A-Z]"' crates/origami_abap/src | tr -d '"' | sort -u | tr '\n' ' ')
cds=$(grep -rohE '"[a-z][a-z_]*[a-z]"' crates/origami_cds/src | tr -d '"' | sort -u | tr '\n' ' ')
# The pipelines above have no pipefail: a moved source directory would give
# an empty list and an unhighlighted playground, so stop here instead.
[ -n "${abap% }" ] || { echo "build.sh: no ABAP keywords found in crates/origami_abap/src" >&2; exit 1; }
[ -n "${cds% }" ] || { echo "build.sh: no CDS keywords found in crates/origami_cds/src" >&2; exit 1; }
printf 'self.KEYWORDS = { abap: "%s", cds: "%s" };\n' "$abap" "$cds" > site/dist/keywords.js
for f in logo logo-dark logo-small logo-small-dark favicon pipeline pipeline-dark benchmark benchmark-dark latency latency-dark review-demo review-demo-dark; do
  cp "docs/assets/$f.svg" site/dist/assets/
done
cp docs/assets/social-preview.png docs/assets/apple-touch-icon.png site/dist/assets/
mkdir -p site/dist/assets/photos
cp docs/assets/photos/*.webp site/dist/assets/photos/
echo "site/dist: $(du -sh site/dist | cut -f1)"
