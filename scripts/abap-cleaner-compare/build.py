#!/usr/bin/env python3
"""Builds site/abap-cleaner.html: origami next to ABAP Cleaner's layout rules.

Needs a built origami (cargo build --release -p abap-origami), a JDK, and
ABAP Cleaner's plugin jars, for example from the VS Code extension:

    ABAP_CLEANER_PLUGINS=~/.vscode/extensions/saposs.abap-cleaner-<v>-<os>/binary/abapcleaner.app/Contents/Eclipse/plugins \\
      python3 scripts/abap-cleaner-compare/build.py [--scan]

--scan first runs every ABAP Cleaner rule alone over the test corpus
(cargo xtask corpus fetch) and rewrites rules.tsv; this takes about half an
hour. Without it, the committed rules.tsv is used.
"""

import html
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
OUT = ROOT / "site" / "abap-cleaner.html"
ORIGAMI = ROOT / "target" / "release" / "origami"
STYLE = "https://github.com/nimble-123/origami/blob/main/docs/STYLE.md"

# ---- examples -----------------------------------------------------------

EXAMPLES = [
    ("01-calls.abap", "Method calls",
     "Both hang the parameters behind the opening bracket and align the "
     "<code>=</code> signs. A call with two or more parameter keywords puts "
     "each keyword on its own line in both tools. origami aligns the "
     "<code>=</code> signs per keyword, ABAP Cleaner across all of them."),
    ("02-long-lines.abap", "Lines longer than the limit",
     "origami breaks every statement that does not fit into the line width: "
     "before <code>AND</code> and <code>OR</code>, and before each SQL clause. "
     "ABAP Cleaner's layout rules align lines that are already broken, but "
     "they leave a long line as it is."),
    ("03-own-breaks.abap", "Your own line breaks",
     "origami does not keep line breaks: whatever fits goes on one line, so "
     "two authors always get the same result. ABAP Cleaner keeps the author's "
     "breaks and aligns around them."),
    ("04-declarations.abap", "Declarations",
     "ABAP Cleaner aligns <code>TYPE</code> across separate "
     "<code>DATA</code> statements and puts each item of a chain on its own "
     "line. origami aligns only inside a chain it has to break, and keeps a "
     "chain that fits on one line."),
    ("05-signatures.abap", "Method signatures",
     "ABAP Cleaner keeps <code>IMPORTING</code> behind the method name and "
     "aligns the types in a table. origami gives each section keyword its own "
     "line and does not align types, so renaming one parameter changes one "
     "line."),
    ("06-comments-blank-lines.abap", "Comments and blank lines",
     "origami puts exactly one space before an end-of-line comment and never "
     "changes comment text, apart from aligning ABAP Doc. ABAP Cleaner keeps aligned comments, adds a space "
     "after <code>\"</code> and adds blank lines, for example after "
     "declarations."),
    ("07-call-function.abap", "CALL FUNCTION",
     "Same structure, two details: origami indents the parameter groups by "
     "four spaces, ABAP Cleaner by two and also aligns the keywords and "
     "<code>=</code> signs across groups."),
    ("08-letter-case.abap", "Letter case",
     "Both write keywords in upper case and identifiers without a lower-case "
     "letter in lower case; an identifier in mixed case keeps it. Literals and "
     "comments are never changed. The only difference here is the blank line "
     "ABAP Cleaner adds after the declarations."),
    ("09-cds-view.asddls", "CDS view entity",
     "Both put each clause of the header and each element on its own line. "
     "ABAP Cleaner also aligns <code>as</code> aliases and adds blank lines "
     "between sections; origami lines up names after <code>key</code> and "
     "keeps single spaces."),
    ("10-aligned-statements.abap", "Aligned statements, COND and SELECT lists",
     "Both tools align consecutive <code>ALIASES</code>, "
     "<code>REDEFINITION</code> and <code>FOR TESTING</code> declarations, "
     "assignments to the same structure, ABAP Doc parameters, the branches of "
     "<code>COND</code> and a <code>SELECT</code> list with aliases, and both "
     "remove the space before the period. The differences: origami indents "
     "the SQL clauses by four spaces and keeps a method signature that fits "
     "on one line; ABAP Cleaner indents by two, aligns the signature in a "
     "table and adds blank lines."),
]

# ---- origami's answer to each ABAP Cleaner rule ----------------------------
# Status: "yes" (origami does the same), "own" (origami covers it with its
# own layout), "no" (layout rule without an origami counterpart). Rules not
# listed change code, comments or names, which origami never does.

MAP = {
    "EMPTY_LINES_WITHIN_METHODS": ("own", ["ABAP-BLANK-001"],
        "Removes blank lines after a block opener and before its closer and keeps at most one in a row; never adds one."),
    "EMPTY_LINES_OUTSIDE_METHODS": ("own", ["ABAP-BLANK-001"],
        "Keeps the blank lines between methods and classes (at most one); does not add any."),
    "EMPTY_LINES_IN_CLASS_DEFINITION": ("own", ["ABAP-BLANK-001"],
        "Does not add blank lines above sections."),
    "ONE_COMMAND_PER_LINE": ("yes", ["ABAP-STMT-001"], ""),
    "SPACES_IN_EMPTY_BRACKETS": ("no", [],
        "Tokens that touch in the input stay together, because in ABAP a space can change the meaning."),
    "CLOSING_BRACKETS_POSITION": ("yes", ["ABAP-CALL-001", "ABAP-VALUE-001"], ""),
    "SPACE_BEFORE_PERIOD": ("yes", ["ABAP-SPACE-001"],
        "Also before commas in SQL lists; a lone period or comma moves up across comment lines."),
    "SPACE_AROUND_COMMENT_SIGN": ("own", ["ABAP-COMMENT-002"],
        "One space before an end-of-line comment; the comment text is never changed."),
    "NEEDLESS_SPACES": ("yes", ["ABAP-COMMENT-002"],
        "Single spaces between tokens; hand-aligned end-of-line comments are not kept."),
    "UPPER_AND_LOWER_CASE": ("yes", ["ABAP-CASE-001", "ABAP-CASE-002"],
        "Keywords in upper case; identifiers without lower-case letters in lower case, mixed case kept."),
    "INSET": ("yes", ["ABAP-BLOCK-001", "ABAP-CLASS-001"], ""),
    "ALIGN_ABAP_DOC": ("yes", ["ABAP-DOC-001"],
        "Only the blanks after the tag and before | change; not aligned if a line would exceed the width."),
    "ALIGN_METHODS_DECLARATION": ("own", ["ABAP-METHODS-001"],
        "Each section keyword on its own line, parameters one level deeper, types not aligned."),
    "ALIGN_METHODS_FOR_TESTING": ("yes", ["ABAP-ALIGN-005"],
        "Not aligned if a line would exceed the width."),
    "ALIGN_METHODS_REDEFINITION": ("yes", ["ABAP-ALIGN-006"],
        "Not aligned if a line would exceed the width."),
    "ALIGN_ALIASES_FOR": ("yes", ["ABAP-ALIGN-007"],
        "Not aligned if a line would exceed the width."),
    "ALIGN_DECLARATIONS": ("own", ["ABAP-DECL-001", "ABAP-ALIGN-003"],
        "TYPE is aligned inside a broken chain, not across separate DATA statements."),
    "ALIGN_ASSIGNMENTS": ("yes", ["ABAP-ALIGN-004"],
        "Not across a chain; not aligned if a line would exceed the width."),
    "ALIGN_WITH_SECOND_WORD": ("own", ["ABAP-INDENT-001"],
        "Continuation lines use a fixed indent of two levels."),
    "ALIGN_CLEAR_FREE_AND_SORT": ("own", ["ABAP-CHAIN-001"],
        "A chain that fits stays on one line; otherwise one item per line, aligned after the colon."),
    "ALIGN_SELECT_CLAUSES": ("own", ["ABAP-SQL-001"],
        "Each clause starts its own line when the statement does not fit."),
    "ALIGN_SELECT_FROM": ("own", ["ABAP-SQL-001"],
        "A JOIN starts its own clause line; ON conditions are not aligned."),
    "ALIGN_SELECT_LISTS": ("own", ["ABAP-SQL-003"],
        "Lists with complex fields one field per line, AS aligned; simple lists on one line if they fit (not derived from the rest of the file)."),
    "ALIGN_PARAMETERS": ("yes", ["ABAP-CALL-001", "ABAP-CALL-002", "ABAP-ALIGN-001", "ABAP-ALIGN-002", "ABAP-VALUE-001"],
        "With several parameter keywords, origami aligns = per keyword."),
    "ALIGN_LOGICAL_EXPRESSIONS": ("own", ["ABAP-COND-001"],
        "Breaks before AND and OR at a fixed indent; operands are not aligned in columns."),
    "LOGICAL_OPERATOR_POSITION": ("yes", ["ABAP-COND-001"], "AND and OR always start the line."),
    "ALIGN_COND_EXPRESSIONS": ("yes", ["ABAP-COND-002"],
        "A COND with one WHEN is joined onto one line when it fits."),
    "ALIGN_FORM_DECLARATION": ("own", ["ABAP-FORM-001"],
        "Each section on its own line, parameters one level deeper."),
    "ALIGN_PERFORM": ("own", [], "A long PERFORM breaks before a section keyword that does not fit."),
    "DDL_SPACES_AROUND_SIGNS": ("own", ["CDS-SPACE-001"],
        "Runs of spaces shrink to one; no space is added or removed next to a sign."),
    "DDL_SPACES_AROUND_BRACKETS": ("own", ["CDS-SPACE-001"],
        "Runs of spaces shrink to one; no space is added or removed next to a bracket."),
    "DDL_ALIGN_ENTITY_PARAMETERS": ("own", ["CDS-HEAD-001"],
        "View parameters go one level deeper; they are not aligned in columns."),
    "DDL_ALIGN_SOURCE_PARAMETERS": ("no", [], "Line breaks inside a header clause are kept as written."),
    "DDL_ALIGN_FUNCTION_PARAMETERS": ("no", [], "Function parameters are not aligned."),
    "DDL_ALIGN_LOGICAL_EXPRESSIONS": ("own", ["CDS-HEAD-001", "DCL-GRANT-001"],
        "A long condition breaks before and / or; operands are not aligned in columns."),
    "DDL_ALIGN_FIELD_LISTS": ("no", [], "Name lists and GROUP BY lists are not aligned."),
    "DDL_ALIGN_DATA_SOURCES": ("own", ["CDS-HEAD-001"],
        "Each join and association starts its own line one level deeper."),
    "DDL_ALIGN_SELECT_LIST": ("own", ["CDS-KEY-001", "CDS-SPACE-001"],
        "Names line up after key; aliases are not aligned."),
    "DDL_EMPTY_LINES_BETWEEN": ("own", ["CDS-BLANK-001"],
        "Keeps blank lines as written (at most one); does not add any."),
    "DDL_EMPTY_LINES_WITHIN": ("own", ["CDS-BLANK-001"],
        "Keeps blank lines as written (at most one); does not add any."),
    "DDL_POSITION_DEFINE": ("yes", ["CDS-UNIT-001"], ""),
    "DDL_POSITION_SELECT": ("yes", ["CDS-HEAD-001"], ""),
    "DDL_POSITION_JOIN": ("yes", ["CDS-HEAD-001"], ""),
    "DDL_POSITION_ASSOCIATION": ("yes", ["CDS-HEAD-001"], ""),
    "DDL_POSITION_BRACES": ("yes", ["CDS-UNIT-001", "CDS-BLOCK-001"], ""),
    "DDL_POSITION_CLAUSES": ("yes", ["CDS-CLAUSE-001"],
        "No blank line is added before UNION or INTERSECT."),
    "DDL_ANNO_LAYOUT": ("own", ["CDS-ANNO-001"],
        "A value goes on one line when it fits, otherwise one item per line as in JSON; not aligned in columns."),
}
NAMES = {"CAMEL_CASE_NAME", "CAMEL_CASE_IN_CDS_TEST", "DDL_CAMEL_CASE_NAME"}

STATUS = {
    "yes": ("Same", "origami does the same"),
    "own": ("Own layout", "origami covers this with its own layout"),
    "no": ("No", "layout rule origami does not have"),
    "goal": ("Not a goal", "changes code, comments or names, which origami never does"),
}

GROUPS = {
    "EMPTY_LINES": "Empty lines", "SPACES": "Spaces", "DECLARATIONS": "Declarations",
    "SYNTAX": "Syntax", "COMMANDS": "Commands", "PRETTY_PRINTER": "Pretty printer",
    "ALIGNMENT": "Alignment", "DDL_ANNOTATIONS": "CDS annotations",
    "DDL_POSITION": "CDS line breaks", "DDL_SPACES": "CDS spaces",
    "DDL_ALIGNMENT": "CDS alignment", "DDL_EMPTY_LINES": "CDS empty lines",
}

# ---- tools ----------------------------------------------------------------


def run(cmd, **kw):
    return subprocess.run(cmd, check=True, text=True, capture_output=True, **kw)


def plugins():
    p = os.environ.get("ABAP_CLEANER_PLUGINS")
    if not p:
        sys.exit("set ABAP_CLEANER_PLUGINS to the plugins folder of ABAP Cleaner (see the docstring)")
    return os.path.expanduser(p)


def compile_java(tmp):
    cls = tmp / "classes"
    run(["javac", "-cp", f"{plugins()}/*", "-d", str(cls), str(HERE / "Compare.java")])
    return f"{cls}:{plugins()}/*"


def cleaner_version():
    jars = sorted(Path(plugins()).glob("com.sap.adt.abapcleaner_*.jar"))
    return jars[-1].stem.split("_", 1)[1] if jars else "?"


def scan(cp, tmp):
    files = []
    for ext in ("*.abap", "*.asddls", "*.asdcls"):
        files += [p for p in (ROOT / "corpus" / "repos").rglob(ext) if ".git" not in p.parts]
    lst = tmp / "files.txt"
    lst.write_text("".join(f"{p}\n" for p in sorted(files)))
    print(f"scanning {len(files)} files, one rule at a time …", file=sys.stderr)
    subprocess.run(["java", "-Xmx6g", "-cp", cp, "Compare", "scan", str(lst), str(HERE / "rules.tsv")], check=True)


def format_both(cp, tmp, name):
    src = HERE / "examples" / name
    before = src.read_text()
    o = tmp / name
    shutil.copy(src, o)
    run([str(ORIGAMI), "--no-config", str(o)])
    c = tmp / ("cleaner-" + name)
    run(["java", "-cp", cp, "Compare", "format", str(src), str(c)])
    return before, c.read_text(), o.read_text()


# ---- HTML -------------------------------------------------------------------

SYNTAX = re.compile(
    r"""(^\*.*|".*|//.*)|('(?:[^']|'')*'?|`(?:[^`]|``)*`?|\|(?:[^|\\]|\\.)*\|?)|\b(\d+)\b|\b([A-Z][A-Z-]*[A-Z])\b"""
)


def paint(line):
    """Same highlighting as site/app.js."""
    out, pos = [], 0
    for m in SYNTAX.finditer(line):
        out.append(html.escape(line[pos:m.start()]))
        cls = "com" if m[1] else "str" if m[2] else "num-lit" if m[3] else "kw"
        out.append(f'<span class="{cls}">{html.escape(m[0])}</span>')
        pos = m.end()
    out.append(html.escape(line[pos:]))
    return "".join(out)


def block(label, text, cls=""):
    code = "\n".join(paint(l) for l in text.rstrip("\n").split("\n"))
    return (f'<div class="cmp-pane {cls}"><div class="pane-label">{label}</div>'
            f'<pre class="sheet code-block" tabindex="0"><code>{code}</code></pre></div>')


def rule_links(ids):
    return " ".join(f'<a href="{STYLE}#{i.lower()}">{i}</a>' for i in ids)


def rules_table(rows):
    counts = {k: 0 for k in STATUS}
    by_group = {}
    for group, rid, name, _layout, text in rows:
        status, ids, note = MAP.get(rid, ("goal", [], ""))
        if status == "goal" and rid in NAMES:
            note = "Changes the case of names."
        counts[status] += 1
        by_group.setdefault(group, []).append((rid, name, int(text), status, ids, note))
    parts = []
    for group, items in by_group.items():
        parts.append(f'<tbody><tr class="group"><th colspan="4" scope="rowgroup">{GROUPS.get(group, group)}</th></tr>')
        for rid, name, text, status, ids, note in items:
            label = STATUS[status][0]
            changed = f'{text:,}' if text else "–"
            parts.append(
                f'<tr><td><span class="rule-name">{html.escape(name)}</span>'
                f'<code class="rule-id">{rid}</code></td>'
                f'<td class="num">{changed}</td>'
                f'<td><span class="badge {status}">{label}</span></td>'
                f'<td>{rule_links(ids)}{" " if ids and note else ""}{html.escape(note)}</td></tr>')
        parts.append("</tbody>")
    return counts, "\n".join(parts)


def page(examples, rows, cleaner, origami, profile_count):
    counts, table = rules_table(rows)
    total = sum(counts.values())
    layout = total - counts["goal"]
    cmp = []
    for (name, title, text), (before, c, o) in zip(EXAMPLES, examples):
        slug = name.split(".")[0][3:]
        cmp.append(f"""
      <section class="cmp" id="{slug}" aria-labelledby="{slug}-title">
        <h3 id="{slug}-title">{title}</h3>
        <p>{text}</p>
        {block("Before", before, "before")}
        <div class="cmp-pair">
          {block("ABAP Cleaner, layout rules", c)}
          {block("origami", o, "ours")}
        </div>
      </section>""")
    legend = "".join(
        f'<li><span class="badge {k}">{v[0]}</span> {v[1]}: <strong>{counts[k]}</strong></li>' for k, v in STATUS.items())
    return f"""<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>origami and ABAP Cleaner</title>
  <meta name="description" content="The same ABAP and CDS code formatted by ABAP Cleaner's layout rules and by origami, and every ABAP Cleaner rule next to origami's rules.">
  <meta name="color-scheme" content="light dark">
  <meta name="theme-color" content="#d5d6db" media="(prefers-color-scheme: light)">
  <meta name="theme-color" content="#16161e" media="(prefers-color-scheme: dark)">
  <link rel="icon" href="assets/logo.svg" type="image/svg+xml">
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link href="https://fonts.googleapis.com/css2?family=Schibsted+Grotesk:wght@400;500;600;700;800&family=Martian+Mono:wght@400;600&display=swap" rel="stylesheet">
  <link rel="stylesheet" href="style.css">
</head>
<body>
  <!-- Generated by scripts/abap-cleaner-compare/build.py; do not edit. -->
  <a class="skip" href="#main">Skip to content</a>

  <header class="top">
    <a class="brand" href="./" aria-label="origami home">
      <picture>
        <source srcset="assets/logo-dark.svg" media="(prefers-color-scheme: dark)">
        <img src="assets/logo.svg" alt="" width="32" height="32">
      </picture>
      <span>origami</span>
    </a>
    <nav aria-label="Sections">
      <a href="#examples">Examples</a>
      <a href="#rules">All rules</a>
      <a href="./#playground">Playground</a>
      <a href="https://github.com/nimble-123/origami">GitHub</a>
    </nav>
  </header>

  <main id="main">
    <section class="page-head" aria-labelledby="title">
      <h1 id="title">origami and ABAP Cleaner</h1>
      <p class="lede">ABAP Cleaner cleans up code: it replaces obsolete statements, removes unused variables and also lays the code out. origami only lays it out. This page compares only the layout: the same code, formatted by ABAP Cleaner's layout rules and by origami.</p>
      <dl class="setup">
        <dt>ABAP Cleaner</dt>
        <dd>{cleaner}, default profile reduced to its {profile_count} layout rules with their default options (<a href="https://github.com/nimble-123/origami/tree/main/scripts/abap-cleaner-compare">how</a>). Checked on 2026-11-23 on the 1,964 ABAP and 144 CDS files of the test corpus: these rules change no code and delete no comment; the only reordering is a closing bracket moved in front of a comment or pragma.</dd>
        <dt>origami</dt>
        <dd>{origami}, default settings.</dd>
        <dt>Both</dt>
        <dd>Line width 120, keywords in upper case, indent 2.</dd>
      </dl>
    </section>

    <section id="examples" class="band" aria-labelledby="examples-title">
      <div class="section-head">
        <h2 id="examples-title">The same code, twice</h2>
        <p>Each input was run through both tools as it is. The differences come from two choices: origami breaks every line that is too long and ignores the author's line breaks, and it aligns columns only inside one statement.</p>
      </div>
      {"".join(cmp)}
    </section>

    <section id="rules" class="band" aria-labelledby="rules-title">
      <div class="section-head">
        <h2 id="rules-title">Every ABAP Cleaner rule</h2>
        <p>ABAP Cleaner {cleaner} has {total} rules. {counts["goal"]} of them change code, comments or names; origami never does that, it checks its own output token by token. Of the other {layout}, origami does {counts["yes"]} the same way and covers {counts["own"]} with its own layout; {counts["no"]} have no counterpart.</p>
      </div>
      <ul class="legend">{legend}</ul>
      <div class="table-wrap" tabindex="0">
        <table class="rules">
          <caption class="sr-only">ABAP Cleaner rules and origami</caption>
          <thead><tr><th scope="col">ABAP Cleaner rule</th><th scope="col" class="num">Files with changed code or comments</th><th scope="col">origami</th><th scope="col">How origami handles it</th></tr></thead>
          {table}
        </table>
      </div>
      <p class="table-note">"Files with changed code or comments": each rule was run alone over the {{corpus}} ABAP and CDS files of the test corpus; the number counts files where more than whitespace and letter case changed. A rule that changes only names (CamelCase) shows –.</p>
    </section>
  </main>

  <footer class="foot">
    <p>ABAP Cleaner is SAP's open-source tool (Apache-2.0), not part of origami. Generated by <code>scripts/abap-cleaner-compare/build.py</code>.</p>
    <p><a href="./">origami home</a> <a href="{STYLE}">STYLE.md</a></p>
  </footer>
</body>
</html>
"""


def main():
    with tempfile.TemporaryDirectory() as t:
        tmp = Path(t)
        cp = compile_java(tmp)
        if "--scan" in sys.argv:
            scan(cp, tmp)
        lines = (HERE / "rules.tsv").read_text().splitlines()
        corpus = f"{int(lines[0].split(chr(9))[1]):,}"
        rows = [l.split("\t") for l in lines[2:]]
        examples = [format_both(cp, tmp, name) for name, *_ in EXAMPLES]
        profile_count = len(run(["java", "-cp", cp, "Compare", "profile"]).stdout.split())
    origami = run([str(ORIGAMI), "--version"]).stdout.strip()
    text = page(examples, rows, cleaner_version(), origami, profile_count).replace("{corpus}", corpus)
    OUT.write_text(text)
    print(f"wrote {OUT.relative_to(ROOT)}", file=sys.stderr)


if __name__ == "__main__":
    main()
