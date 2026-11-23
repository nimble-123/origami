# origami – ABAP & CDS formatter

Layout-only formatter for ABAP and the CDS family: classes, interfaces,
programs, CDS views (DDLS), metadata extensions (DDLX), access controls (DCL),
behavior definitions (BDEF) and service definitions (SRVD).

It works on abapGit files on disk and on the documents of SAP's ADT for VS Code.
origami changes layout only. After every run it checks that only whitespace,
line breaks and letter case changed, and it never touches a document with a
structural error.

## Setup

Set origami as the default formatter, for example in your user settings:

```json
"[abap]": { "editor.defaultFormatter": "origami-dev.abap-origami" },
"[acds]": { "editor.defaultFormatter": "origami-dev.abap-origami" },
"editor.formatOnSave": true
```

ADT for VS Code registers its own formatter for `abap` and `acds`, so the
`editor.defaultFormatter` setting is needed.

For code that lives in an SAP system, consider
`"editor.formatOnSaveMode": "modifications"`, so saving one method does not
reformat the whole class.

## Preview Changes

"origami: Preview Changes" (command palette) shows what formatting would
change, in VS Code's diff view: the document as it is now on the left, the
formatted text on the right. Nothing is changed yet.

"origami: Apply Preview" (the check mark in the diff view's title bar)
applies the result; the document stays unsaved. If you edited the document
after opening the preview, Apply formats the current text instead, so your
later edits are kept.

If the document has a structural error, no diff opens and the problem is
shown in the Problems panel.

## Binary

The extension uses, in this order:

1. the machine setting `origami.path`;
2. an `origami` on `PATH` that satisfies `required_version` of the project's
   `origami.toml`;
3. the bundled binary.

In untrusted workspaces only the bundled binary is used.

## Settings

* `origami.path`: path to an origami binary (machine scope).
* `origami.trace`: log every command line and its stderr to the output
  channel `origami`.

Problems from origami (structural errors, quarantined statements) appear in
the Problems panel.
