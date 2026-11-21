# Spike S2: ADT for VS Code behavior

**Status:** partly answered from the installed extension manifest
(`sapse.adt-vscode` 1.2.1); the rest needs a connected system.

**Decisions it feeds:** URI-based language hints and file names in
`editors/vscode/src/uri.ts`; how the extension edits `abap:` documents.

## Known from the extension manifest (2026-11-21)

* Source documents use the URI scheme `abap:` and are virtual (not on disk).
* Language IDs: `abap` for ABAP sources, `acds` for CDS-family sources.
* The extension contributes an MCP server (`com.sap.adt/mcp`) and registers
  document formatting through its language server.

## Questions for a connected system

| # | Question | Answer |
|---|---|---|
| 1 | Does `acds` cover DDLS, DDLX, DCLS, BDEF and SRVD? Which language ID do simple types (DRTY) get? | |
| 2 | What does an `abap:` URI look like per object type? Does the path encode the object type (for example `/bo/behaviordefinitions/`)? | |
| 3 | Can a third-party `DocumentFormattingEditProvider` edit an `abap:` document? Locked and unlocked? | |
| 4 | Does ADT's own "Format Document" need a system connection? What does it change? | |
| 5 | Does saving an unchanged document create an inactive version or a transport entry? | |

## Procedure

1. Install the origami development build of the extension (M4), or use a
   minimal test extension that registers a formatter for `abap` and `acds`.
2. Open one object of each type; log `document.uri.toString()` and
   `document.languageId`.
3. Run "Format Document" with origami selected; record whether the edit is
   applied, and what happens on save.

## Decision

To be written after the experiment, then copied into Open Question 2 of
`docs/designs/origami.md` and item 90 (`uri.ts`).
