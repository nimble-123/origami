# Adopting origami in a team

This guide takes a repository from "no formatter" to "every commit is
formatted" in five steps. Plan about an hour, most of it for the review of the
one-time mass format.

## 1. Pin the version

Add `origami.toml` at the repository root (next to `.git`):

```toml
required_version = ">=0.1, <0.2"
```

Every origami run, the VS Code extension included, refuses to format with a
version outside this range (E020). This keeps CI, editors and agents on the
same output. Raise the range in a pull request when you move to a new version.

While origami is 0.x, any release can change the formatted output, patch
releases included (0.1.1 did), and the changelog lists each change with its
rule ID. The range above takes new patch releases; to keep the output of one
release exactly, pin it (`required_version = "=0.1.1"`). To upgrade: raise the
range, run `origami` once, review the diff by sampling, and commit it on its
own (step 2). From 1.0, new and changed rules come behind `--preview` first and
become the default only with a new style edition; see [STYLE.md](STYLE.md),
"Stability".

origami has two styles ([STYLE.md](STYLE.md), "Styles"). The default,
`aligned`, follows ABAP Cleaner's layout: parameter keywords, declarations and
method signatures aligned in columns, continuation lines indented by one
level, and blank lines between sections. `compact` is the layout of origami
0.3; a team that already formatted with 0.3 and wants to keep that layout
sets it once:

```toml
style = "compact"
```

## 2. Format everything once, in its own commit

Look first: `origami --review` shows every change origami would make, with a
note below each line break it introduces ("ABAP-CALL-001: one parameter per
line, because the call would end at column 134 (limit 120)"). It writes
nothing. A plain diff, for example in a diff viewer, comes from
`origami --check --diff | delta`. In VS Code, "origami: Preview Changes" shows
the formatting of the open file in the diff view.

```sh
origami                     # formats all supported files below .
git commit -am "Format with origami"
```

Code written in upper case gets a large first diff: origami writes ABAP
identifiers without a lower-case letter in lower case (STYLE ABAP-CASE-002).

Review the diff by sampling, not line by line: origami re-lexes its output
and refuses to write a file if anything but whitespace, line breaks or
letter case changed, and the corpus tests check the same on 1,800 public
files.

Then hide this commit from `git blame`:

```sh
git rev-parse HEAD >> .git-blame-ignore-revs
git config blame.ignoreRevsFile .git-blame-ignore-revs
git commit -am "Ignore the mass format in git blame"
```

GitHub uses `.git-blame-ignore-revs` automatically.

## 3. Check in CI

```sh
origami --check --diagnostics json
```

Exit code 1 means a file would change; 2 means a structural error (the file
is left as it is); 3 means a configuration problem. The full list is in
[CLI.md](CLI.md). Use `--check --diff` to show what would change.

Add `--strict` only when you want warnings to fail the build. W003 ("statement
not recognized; formatted generically") is a warning, so `--strict` also fails
on statements the grammar does not cover yet.

## 4. Check before each commit

With the [pre-commit](https://pre-commit.com) framework:

```yaml
repos:
  - repo: https://github.com/nimble-123/origami
    rev: v0.3.1 # x-release-please-version
    hooks:
      - id: origami        # check only
      # - id: origami-fix  # or rewrite the staged files
```

Without it, a plain git hook works:

```sh
#!/bin/sh
git diff --cached --name-only --diff-filter=ACM \
  | grep -E '\.(abap|asddls|asddlxs|asdcls|asbdef|srvdsrv|acds)$' \
  | xargs -r origami --check
```

## 5. Format in the editor

Install the VS Code extension and make it the default formatter, because ADT
for VS Code registers its own formatter for the same languages:

```json
"[abap]": { "editor.defaultFormatter": "nimble-123.abap-origami" },
"[acds]": { "editor.defaultFormatter": "nimble-123.abap-origami" },
"editor.formatOnSave": true
```

For code that lives in an SAP system (ADT for VS Code), consider

```json
"editor.formatOnSaveMode": "modifications"
```

so that saving one method does not reformat the whole class and drag
unrelated lines into the next transport.

## Teammates with other tools

origami has its own fixed style. A teammate who presses Shift+F1 in Eclipse,
or runs ABAP Cleaner on save, changes the layout back, and the CI check then
fails. Agree on one of these:

* switch off the pretty printer and ABAP Cleaner on save in Eclipse, and run
  origami before committing; or
* let CI be the judge: whoever's commit fails the check runs `origami` and
  commits again.

## AI agents

Agents should pipe source through stdin:

```sh
origami --stdin-filepath zcl_foo.clas.abap --diagnostics json < in.abap > out.abap
```

On any error stdout stays empty and the exit code is not 0, so an agent never
writes an empty object back. With the ADT MCP server, skip the write-back when
the JSON says `"changed": false`.
