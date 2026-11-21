#!/usr/bin/env bash
# End-to-end CLI benchmark against the G4 targets.
#
# 1. Copy the first 1,000 supported corpus files (path order) to
#    target/bench-input/ and format them there, so `--check` exits 0.
# 2. hyperfine: one median-sized file per language (p95 target 50 ms) and the
#    1,000-file set (target 1 s).
# 3. cargo xtask bench-check compares the results with G4 (20% tolerance).
set -euo pipefail

cd "$(dirname "$0")/.."
cargo build --release -q -p abap-origami
BIN="$PWD/target/release/origami"
OUT="$PWD/target/bench-input"
RES="$PWD/target/bench-results"
rm -rf "$OUT" "$RES"
mkdir -p "$OUT/all" "$RES"

# Supported files in path order.
mapfile -t FILES < <(find corpus/repos -path '*/.git' -prune -o -type f \
  \( -name '*.abap' -o -name '*.asddls' -o -name '*.asddlxs' -o -name '*.asdcls' \
     -o -name '*.asbdef' -o -name '*.srvdsrv' \) -print | LC_ALL=C sort)
if [ "${#FILES[@]}" -eq 0 ]; then
  echo "no corpus files: run 'cargo xtask corpus fetch'" >&2
  exit 1
fi

i=0
for f in "${FILES[@]:0:1000}"; do
  i=$((i + 1))
  cp "$f" "$OUT/all/$(printf '%04d' "$i")_$(basename "$f")"
done
"$BIN" --no-config "$OUT/all" >/dev/null 2>&1 || true

# One median-sized file per language.
for ext in clas.abap ddls.asddls ddlx.asddlxs dcls.asdcls bdef.asbdef srvd.srvdsrv; do
  mapfile -t L < <(for f in "${FILES[@]}"; do
    case "$f" in *."$ext") printf '%s %s\n' "$(wc -l < "$f")" "$f";; esac
  done | sort -n | awk '{print $2}')
  [ "${#L[@]}" -eq 0 ] && continue
  m="${L[$(( ${#L[@]} / 2 ))]}"
  cp "$m" "$OUT/single.$ext"
  "$BIN" --no-config "$OUT/single.$ext" >/dev/null 2>&1 || true
  hyperfine --warmup 5 --runs 50 --ignore-failure --export-json "$RES/single-$ext.json" \
    "$BIN --no-config --check $OUT/single.$ext"
done

hyperfine --warmup 2 --runs 10 --ignore-failure --export-json "$RES/thousand.json" \
  "$BIN --no-config --check $OUT/all"

cargo xtask bench-check "$RES"/*.json
