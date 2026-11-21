#!/usr/bin/env bash
# End-to-end CLI benchmark against the G4 targets. Works with bash 3.2 (macOS).
#
# 1. Copy the first 1,000 supported corpus files (path order) to
#    target/bench-input/all and format them there, so `--check` exits 0.
# 2. hyperfine: one median-sized file per language (p95 target 50 ms,
#    including process start) and the 1,000-file set (target 1 s).
# 3. cargo xtask bench-check compares the results with G4 (20% tolerance).
set -euo pipefail

cd "$(dirname "$0")/.."
cargo build --release -q -p abap-origami
BIN="$PWD/target/release/origami"
OUT="$PWD/target/bench-input"
RES="$PWD/target/bench-results"
rm -rf "$OUT" "$RES"
mkdir -p "$OUT/all" "$RES"

LIST="$OUT/files.txt"
find corpus/repos -path '*/.git' -prune -o -type f \
  \( -name '*.abap' -o -name '*.asddls' -o -name '*.asddlxs' -o -name '*.asdcls' \
     -o -name '*.asbdef' -o -name '*.srvdsrv' \) -print | LC_ALL=C sort > "$LIST"
if [ ! -s "$LIST" ]; then
  echo "no corpus files: run 'cargo xtask corpus fetch'" >&2
  exit 1
fi

i=0
while IFS= read -r f; do
  i=$((i + 1))
  [ "$i" -gt 1000 ] && break
  cp "$f" "$OUT/all/$(printf '%04d' "$i")_$(basename "$f")"
done < "$LIST"
"$BIN" --no-config "$OUT/all" >/dev/null 2>&1 || true

for ext in clas.abap ddls.asddls ddlx.asddlxs dcls.asdcls bdef.asbdef srvd.srvdsrv; do
  # Median-sized file of this type.
  m=$(grep "\.${ext}\$" "$LIST" | while IFS= read -r f; do
        printf '%s %s\n' "$(wc -l < "$f" | tr -d ' ')" "$f"
      done | sort -n | awk '{ a[NR] = $2 } END { if (NR) print a[int((NR + 1) / 2)] }')
  [ -z "$m" ] && continue
  cp "$m" "$OUT/single.$ext"
  "$BIN" --no-config "$OUT/single.$ext" >/dev/null 2>&1 || true
  hyperfine -N --warmup 5 --runs 50 --ignore-failure --style none \
    --export-json "$RES/single-$ext.json" \
    "$BIN --no-config --check $OUT/single.$ext" >/dev/null
done

hyperfine -N --warmup 2 --runs 10 --ignore-failure --style none \
  --export-json "$RES/thousand.json" \
  "$BIN --no-config --check $OUT/all" >/dev/null

cargo xtask bench-check "$RES"/*.json
