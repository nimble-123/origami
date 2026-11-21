// End-to-end tests inside a real VS Code: the extension formats a file
// through the standard format-document API, and reports structural errors
// as diagnostics without changing the file.

import * as assert from "node:assert/strict";
import * as fs from "node:fs";
import * as path from "node:path";
import * as vscode from "vscode";

type Test = [name: string, body: () => Promise<void>];

async function openAndFormat(name: string, text: string): Promise<vscode.TextDocument> {
  const dir = process.env.ORIGAMI_E2E_WORKSPACE;
  assert.ok(dir, "ORIGAMI_E2E_WORKSPACE not set");
  const file = path.join(dir, name);
  fs.writeFileSync(file, text);
  const doc = await vscode.workspace.openTextDocument(file);
  await vscode.window.showTextDocument(doc);
  const edits = await vscode.commands.executeCommand<vscode.TextEdit[]>(
    "vscode.executeFormatDocumentProvider",
    doc.uri,
    { tabSize: 2, insertSpaces: true },
  );
  if (edits && edits.length > 0) {
    const we = new vscode.WorkspaceEdit();
    we.set(doc.uri, edits);
    assert.ok(await vscode.workspace.applyEdit(we), "applyEdit failed");
  }
  return doc;
}

async function waitFor<T>(get: () => T | undefined, ms = 5000): Promise<T> {
  const end = Date.now() + ms;
  for (;;) {
    const v = get();
    if (v !== undefined) {
      return v;
    }
    if (Date.now() > end) {
      throw new Error("timed out");
    }
    await new Promise((r) => setTimeout(r, 50));
  }
}

const tests: Test[] = [
  [
    "activates in an abapGit-style workspace",
    async () => {
      const ext = vscode.extensions.all.find((e) => e.packageJSON.name === "abap-origami");
      assert.ok(ext, "extension not installed");
      await waitFor(() => (ext.isActive ? true : undefined));
    },
  ],
  [
    "formats an ABAP file",
    async () => {
      const doc = await openAndFormat("z_e2e.prog.abap", "data lv_a type i.\nlv_a   =   1.\n");
      assert.equal(doc.getText(), "DATA lv_a TYPE i.\nlv_a = 1.\n");
    },
  ],
  [
    "formats a CDS view",
    async () => {
      const doc = await openAndFormat(
        "zi_e2e.ddls.asddls",
        "define view entity ZI_E2E as select from t {\nkey id,\nname\n}\n",
      );
      assert.equal(
        doc.getText(),
        "define view entity ZI_E2E\n  as select from t\n{\n  key id,\n      name\n}\n",
      );
    },
  ],
  [
    "leaves a file with a structural error unchanged and reports it",
    async () => {
      const text = "lv_a   =   1\n";
      const doc = await openAndFormat("z_e2e_err.prog.abap", text);
      assert.equal(doc.getText(), text);
      const diags = await waitFor(() => {
        const d = vscode.languages.getDiagnostics(doc.uri);
        return d.length > 0 ? d : undefined;
      });
      assert.equal(diags[0].source, "origami");
      assert.equal(String(diags[0].code), "E003");
    },
  ],
  [
    "is idempotent: a formatted file gets no edits",
    async () => {
      const doc = await openAndFormat("z_e2e_ok.prog.abap", "lv_a = 1.\n");
      const edits = await vscode.commands.executeCommand<vscode.TextEdit[]>(
        "vscode.executeFormatDocumentProvider",
        doc.uri,
        { tabSize: 2, insertSpaces: true },
      );
      assert.equal(edits?.length ?? 0, 0);
    },
  ],
];

// Entry point called by the VS Code test host.
export async function run(): Promise<void> {
  let failed = 0;
  for (const [name, body] of tests) {
    try {
      await body();
      console.log(`ok   ${name}`);
    } catch (e) {
      failed += 1;
      console.log(`FAIL ${name}\n${e instanceof Error ? e.stack : String(e)}`);
    }
  }
  console.log(`e2e: ${tests.length - failed} passed, ${failed} failed`);
  if (failed > 0) {
    throw new Error(`${failed} e2e test(s) failed`);
  }
}
