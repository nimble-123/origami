#!/bin/sh
# Captures `origami --review` on the website's sample file, with colors, into
# site/samples/review.ansi. site/fill.mjs turns it into the terminal on the
# page. Run it again whenever the formatter's output changes:
#   cargo build --release -p abap-origami && sh site/samples/review.sh
set -eu
cd "$(dirname "$0")"
bin=../../target/release/origami
# Colors as in a terminal; the plain, human format (not the agent format).
env -u CLAUDECODE CLICOLOR_FORCE=1 FORCE_COLOR=1 "$bin" --review zcl_flight_check.clas.abap > review.ansi 2>&1 || [ $? -eq 1 ]
echo "wrote site/samples/review.ansi"
