#!/bin/sh
# Builds the website into site/dist: the static pages, origami compiled to
# WebAssembly, and the code views the page shows without scripting (filled
# from that WebAssembly by site/fill.mjs, so they always match the
# formatter). Serve it with any static server, e.g.
#   sh site/build.sh && python3 -m http.server -d site/dist 8000
# The link preview image (docs/assets/social-preview.png) is rendered from
# site/og/og.html with a headless browser at 1280x640.
set -eu
cd "$(dirname "$0")/.."
rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true
cargo build --release -p origami_wasm --target wasm32-unknown-unknown
rm -rf site/dist
mkdir -p site/dist/assets site/dist/samples
cp site/index.html site/abap-cleaner.html site/style.css site/dist/
cp site/app.js site/format.js site/hero.js site/chapters.js site/render.js site/samples.js site/worker.js site/nav.js site/crane.js site/dist/
cp site/samples/zcl_flight_check.clas.abap site/dist/samples/
# three.js (MIT, see THIRD_PARTY_NOTICES.md) for the crane in the closing
# section, loaded by app.js only when that section comes near.
mkdir -p site/dist/vendor
cp site/vendor/three.module.min.js site/vendor/three.LICENSE site/dist/vendor/
mkdir -p site/dist/fonts
cp site/fonts/*.woff2 site/dist/fonts/
cp target/wasm32-unknown-unknown/release/origami_wasm.wasm site/dist/origami.wasm
# Keywords for the highlighting: the words origami's parsers know.
abap=$(grep -rohE '"[A-Z][A-Z-]*[A-Z]"' crates/origami_abap/src | tr -d '"' | sort -u | tr '\n' ' ')
cds=$(grep -rohE '"[a-z][a-z_]*[a-z]"' crates/origami_cds/src | tr -d '"' | sort -u | tr '\n' ' ')
# The pipelines above have no pipefail: a moved source directory would give
# an empty list and an unhighlighted playground, so stop here instead.
[ -n "${abap% }" ] || { echo "build.sh: no ABAP keywords found in crates/origami_abap/src" >&2; exit 1; }
[ -n "${cds% }" ] || { echo "build.sh: no CDS keywords found in crates/origami_cds/src" >&2; exit 1; }
printf 'self.KEYWORDS = { abap: "%s", cds: "%s" };\n' "$abap" "$cds" > site/dist/keywords.js
for f in logo-dark logo-small-dark favicon; do
  cp "docs/assets/$f.svg" site/dist/assets/
done
cp docs/assets/social-preview.png docs/assets/apple-touch-icon.png docs/assets/paper-fibre.webp site/dist/assets/
# The formatted code the page shows without scripting, from origami.wasm.
node site/fill.mjs site/dist
echo "site/dist: $(du -sh site/dist | cut -f1)"
