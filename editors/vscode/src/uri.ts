// Names and language hints for documents. For `file:` documents the absolute
// path is passed, so origami detects the language from the abapGit file name
// and finds origami.toml. Other schemes (ADT for VS Code uses `abap:`) get a
// synthesized name and a language hint: `abap` for ABAP, `cds` for the CDS
// family (origami detects DDLS, DDLX, DCL, BDEF or SRVD from the content).
// Pending spike S2: if `abap:` URIs encode the object type, derive the exact
// abapGit name here.

import * as vscode from "vscode";
import * as path from "node:path";

export interface Target {
  name: string;
  language?: "abap" | "cds";
  useConfig: boolean;
}

export function targetFor(document: vscode.TextDocument): Target {
  if (document.uri.scheme === "file") {
    return { name: document.uri.fsPath, useConfig: true };
  }
  const base = path.posix.basename(document.uri.path) || `document.${document.languageId}`;
  if (document.languageId === "abap") {
    return { name: base, language: "abap", useConfig: false };
  }
  return { name: base, language: "cds", useConfig: false };
}
