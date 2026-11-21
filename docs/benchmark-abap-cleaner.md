# Performance: origami vs ABAP Cleaner

Measured on 2026-11-21. Apple M4 (10 cores), macOS, Temurin JDK 27.0.1,
ABAP Cleaner 1.29.2 (GitHub release), origami at commit `ae9dc0f`.

## The two tools do different work

ABAP Cleaner applies about 100 cleanup rules, several of which change code
(for example `NEW` instead of `CREATE OBJECT`). origami only changes layout.
The numbers below compare the time to process the same files, not the same
work. The "parse only" row is the closest comparison: ABAP Cleaner parses
and checks the code without applying rules.

## Input

All 1,652 `.abap` files of the test corpus (`corpus/manifest.toml`),
426,831 lines. ABAP Cleaner processed all of them without errors.

## How ABAP Cleaner was run

The native launcher of the macOS download has an invalid code signature, so
macOS Gatekeeper rejects it. ABAP Cleaner was therefore run through its Java
API (`Job.createForSingleCodeDocument`, profiles `Profile.createDefault()`
and `Profile.createEssential()`) with the harness in
`scripts/abap-cleaner-bench/Bench.java`, on the plugin jars of the release.
This skips the Eclipse/OSGi start of the real launcher, so the cold-start
number for ABAP Cleaner is a lower bound. The harness runs on one thread.

## Results

Whole corpus, warm (third of three passes; ABAP Cleaner after JIT warm-up):

| Tool | Time | Per file | origami is faster by |
|---|---:|---:|---:|
| ABAP Cleaner, default profile | 10.44 s | 6.3 ms | 17× |
| ABAP Cleaner, essential profile | 6.69 s | 4.0 ms | 11× |
| ABAP Cleaner, parse only | 1.93 s | 1.2 ms | 3.1× |
| origami, 1 thread (CLI, `--check`, incl. file reading) | 0.62 s | 0.38 ms | – |
| origami, 10 threads (CLI, `--check`) | 0.14 s | – | 77× vs default |

One file, cold (a new process per call, as in a pre-commit hook or a
format-on-save without a running server), median-sized class (81 lines):

| Tool | Time |
|---|---:|
| ABAP Cleaner (JVM start + one file, default profile) | 356 ms |
| origami | 1.7 ms |

ABAP Cleaner 1.29 also has a daemon mode that keeps the JVM running; with
it, a call costs about the warm per-file time above (4–6 ms) plus the
request round trip.

## Reproduce

```sh
# ABAP Cleaner release, unpacked; P = .../abapcleaner.app/Contents/Eclipse/plugins
find corpus/repos -path '*/.git' -prune -o -type f -name '*.abap' -print \
  | LC_ALL=C sort | sed "s#^#$PWD/#" > /tmp/abap-files.txt
cd scripts/abap-cleaner-bench
javac -cp "$P/*" -d /tmp/acb Bench.java
java -Xmx4g -cp "/tmp/acb:$P/*" Bench default /tmp/abap-files.txt 3   # also: essential, parse

# origami: copy the same files into one folder, then
RAYON_NUM_THREADS=1 target/release/origami --no-config --check <folder>
```
