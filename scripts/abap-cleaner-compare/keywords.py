#!/usr/bin/env python3
"""Finds ABAP keywords that origami does not recognize as keywords.

The code of every ABAP file in the test corpus (outside literals, comments
and pragmas) is written in lower case, then formatted by origami and by
ABAP Cleaner's layout rules (see README.md). Both upper-case the keywords
they recognize. A word that ABAP Cleaner upper-cases and origami leaves in
lower case is a keyword origami's parser does not mark: it keeps its case
today and would be lower-cased by a rule that re-cases identifiers.

    ABAP_CLEANER_PLUGINS=… python3 scripts/abap-cleaner-compare/keywords.py [--top N]

Writes target/keyword-gaps.tsv (word, occurrences, files, example line); it
quotes corpus code, so it stays out of the repository. Prints a summary.
Needs a release build of origami.
"""

import collections
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
ORIGAMI = ROOT / "target" / "release" / "origami"
WORD = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")


def pieces(line):
    """(is_code, text) pieces of one line of ABAP."""
    if line.startswith("*"):
        return [(False, line)]
    out, i, n = [], 0, len(line)
    mode, depth = "code", 0  # depth: open `{` inside string templates
    while i < n:
        ch = line[i]
        if mode == "code":
            if ch == '"':
                out.append((False, line[i:]))
                break
            if ch in "'`":
                j = i + 1
                while j < n and not (line[j] == ch and not line.startswith(ch * 2, j)):
                    j += 2 if line.startswith(ch * 2, j) else 1
                out.append((False, line[i:j + 1]))
                i = j + 1
                continue
            if ch == "|":
                out.append((False, ch))
                mode = "tmpl"
            elif ch == "}" and depth:
                out.append((False, ch))
                depth -= 1
                mode = "tmpl"
            elif line.startswith("##", i):
                m = re.match(r"##\w+(\[[^\]]*\])*", line[i:])
                k = len(m.group(0)) if m else 2
                out.append((False, line[i:i + k]))
                i += k
                continue
            else:
                out.append((True, ch))
            i += 1
        else:  # literal text of a string template
            if ch == "\\" and i + 1 < n:
                out.append((False, line[i:i + 2]))
                i += 2
                continue
            if ch == "{":
                depth += 1
                mode = "code"
            elif ch == "|":
                mode = "code"
            out.append((False, ch))
            i += 1
    return out


def lower_code(src):
    return "\n".join("".join(t.lower() if c else t for c, t in pieces(l)) for l in src.split("\n"))


def words(src):
    """(word, line) for every word of code."""
    out = []
    for y, line in enumerate(src.split("\n")):
        code = "".join(t if c else " " * len(t) for c, t in pieces(line))
        out += [(m.group(0), y) for m in WORD.finditer(code)]
    return out


def embedded_lines(src):
    """Lines of macro bodies, AMDP methods and EXEC SQL, which origami copies
    byte for byte."""
    out, end = set(), None
    for y, line in enumerate(src.split("\n")):
        code = "".join(t for c, t in pieces(line) if c).strip()
        if end is None:
            if re.match(r"define\b", code):
                end = r"end-of-definition\b"
            elif re.match(r"exec\s+sql\b", code):
                end = r"endexec\b"
            elif re.match(r"method\b.*\bby\s+database\b", code):
                end = r"endmethod\b"
        if end is not None:
            out.add(y)
            if re.search(end, code):
                end = None
    return out


def main():
    plugins = os.environ.get("ABAP_CLEANER_PLUGINS")
    if not plugins:
        sys.exit("set ABAP_CLEANER_PLUGINS (see README.md)")
    plugins = os.path.expanduser(plugins)
    top = int(sys.argv[sys.argv.index("--top") + 1]) if "--top" in sys.argv else 40
    files = sorted(p for p in (ROOT / "corpus" / "repos").rglob("*.abap") if ".git" not in p.parts)
    with tempfile.TemporaryDirectory() as t:
        tmp = Path(t)
        low, ori, cle = tmp / "low", tmp / "origami", tmp / "cleaner"
        for d in (low, ori, cle):
            d.mkdir()
        names = []
        for i, f in enumerate(files):
            name = f"{i}.{f.name.split('.', 1)[1]}"
            (low / name).write_text(lower_code(f.read_text(errors="replace")))
            names.append(name)
        shutil.copytree(low, ori, dirs_exist_ok=True)
        run = subprocess.run([str(ORIGAMI), "--no-config", "--diagnostics", "json", str(ori)],
                             capture_output=True, text=True)
        # Lines of statements without grammar (W003) and units left as written
        # (W002), which keep their case.
        w003 = collections.defaultdict(set)
        for f in json.loads(run.stderr)["files"]:
            for d in f["diagnostics"]:
                if d["code"] in ("W002", "W003"):
                    r = d["range"]
                    w003[Path(f["path"]).name].update(range(r["start"]["line"] - 1, r["end"]["line"]))
        lst = tmp / "files.txt"
        lst.write_text("".join(f"{low / n}\n" for n in names))
        cls = tmp / "classes"
        cp = f"{cls}:{plugins}/*"
        subprocess.run(["javac", "-cp", f"{plugins}/*", "-d", str(cls), str(HERE / "Compare.java")], check=True)
        subprocess.run(["java", "-Xmx4g", "-cp", cp, "Compare", "format-all", str(lst), str(cle)], check=True)

        gaps = collections.Counter()
        skipped = collections.Counter()
        gap_files = collections.defaultdict(set)
        examples = {}
        extra = collections.Counter()
        compared = 0
        for i, n in enumerate(names):
            src = (low / n).read_text()
            c = (cle / f"{i}.abap").read_text()
            o = (ori / n).read_text()
            if c == src:
                continue  # ABAP Cleaner failed and copied the input
            wo, wc, wi = words(o), words(c), words(src)
            embedded = embedded_lines(src)
            if [w.lower() for w, _ in wo] != [w.lower() for w, _ in wc]:
                continue
            compared += 1
            olines = o.split("\n")
            for (a, y), (b, _), (_, yi) in zip(wo, wc, wi):
                if b.isupper() and not a.isupper() and yi in embedded:
                    skipped["embedded"] += 1
                elif b.isupper() and not a.isupper() and yi in w003[n]:
                    skipped["W003"] += 1
                elif b.isupper() and not a.isupper():
                    gaps[b] += 1
                    gap_files[b].add(n)
                    examples.setdefault(b, f"{files[i].relative_to(ROOT / 'corpus' / 'repos')}: {olines[y].strip()}")
                elif a.isupper() and not b.isupper():
                    extra[a] += 1

    rows = ["word\toccurrences\tfiles\texample"] + [
        f"{w}\t{k}\t{len(gap_files[w])}\t{examples[w]}" for w, k in gaps.most_common()]
    (ROOT / "target" / "keyword-gaps.tsv").write_text("\n".join(rows) + "\n")
    print(f"{compared} of {len(files)} files compared")
    print(f"not counted: {skipped['W003']} in statements without grammar or left as written (W003, W002), "
          f"{skipped['embedded']} in macros, AMDP and EXEC SQL (copied as written)")
    print(f"keywords of recognized statements that origami leaves as written: "
          f"{sum(gaps.values())} occurrences of {len(gaps)} words")
    for w, k in gaps.most_common(top):
        print(f"  {w:<16} {k:>5}  {examples[w][:110]}")
    print(f"words origami upper-cases and ABAP Cleaner does not: {sum(extra.values())} occurrences of {len(extra)} words")
    for w, k in extra.most_common(10):
        print(f"  {w:<16} {k:>5}")


if __name__ == "__main__":
    main()
