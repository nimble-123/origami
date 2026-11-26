// origami VS Code extension: a document formatter for ABAP and the CDS family,
// for abapGit files on disk and ADT for VS Code documents (`abap:` scheme).

import * as vscode from "vscode";
import * as fs from "node:fs";
import { BinaryResolver } from "./binary";
import { formatDocument, FORMAT_TIMEOUT_MS, TextOutcome } from "./format";
import { toEditorRange } from "./diagnosticsModel";
import { PreviewManager } from "./preview";
import { SCHEME as PREVIEW_SCHEME } from "./previewUri";

const SELECTOR: vscode.DocumentSelector = [
  { language: "abap" },
  { language: "acds" },
  { scheme: "file", pattern: "**/*.{abap,asddls,asddlxs,asdcls,asbdef,srvdsrv,acds}" },
];

export function activate(context: vscode.ExtensionContext): void {
  const output = vscode.window.createOutputChannel("origami");
  // Errors are always logged; lines marked `verbose` (command lines, routine
  // stderr) only with `origami.trace`.
  const log = (m: string, verbose = false) => {
    if (!verbose || vscode.workspace.getConfiguration("origami").get<boolean>("trace")) {
      output.appendLine(m);
    }
  };
  const diagnostics = vscode.languages.createDiagnosticCollection("origami");
  const status = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
  status.command = "origami.showOutput";
  status.text = "origami";
  const resolver = new BinaryResolver(context.extensionPath, log);
  // VSIX extraction does not always keep the executable bit.
  if (process.platform !== "win32") {
    try {
      fs.chmodSync(resolver.bundled(), 0o755);
    } catch {
      // No bundled binary in this package; PATH or origami.path is used.
    }
  }

  const setStatus = (text: string, tooltip: string) => {
    status.text = text;
    status.tooltip = tooltip;
    status.show();
  };

  const provider: vscode.DocumentFormattingEditProvider &
    vscode.DocumentRangeFormattingEditProvider = {
    provideDocumentFormattingEdits(document, _options, token) {
      return run(document, token, []);
    },
    // Format Selection, and "format on save: modifications" (only the lines
    // changed since the last commit).
    provideDocumentRangeFormattingEdits(document, range, _options, token) {
      return run(document, token, [range]);
    },
    provideDocumentRangesFormattingEdits(document, ranges, _options, token) {
      return run(document, token, ranges);
    },
  };

  // Problems panel and status bar after formatting or a preview.
  function reportOutcome(
    document: vscode.TextDocument,
    outcome: Pick<TextOutcome, "run" | "status">,
    timeoutMs: number,
  ): void {
    // Diagnostics from every run, including warnings from successful runs.
    const items = (outcome.run?.files[0]?.diagnostics ?? []).map((d) => {
      const r = toEditorRange(d);
      const severity =
        d.severity === "warning"
          ? vscode.DiagnosticSeverity.Warning
          : d.severity === "info"
            ? vscode.DiagnosticSeverity.Information
            : vscode.DiagnosticSeverity.Error;
      const diag = new vscode.Diagnostic(
        new vscode.Range(r.startLine, r.startCharacter, r.endLine, r.endCharacter),
        d.message,
        severity,
      );
      diag.source = "origami";
      diag.code = d.code;
      return diag;
    });
    // Unknown-statement warnings (W003) would flood the Problems panel.
    diagnostics.set(document.uri, items.filter((d) => d.code !== "W003"));
    switch (outcome.status) {
      case "ok":
        setStatus("origami", "Formatted with origami");
        break;
      case "error":
        setStatus("origami: error – see Problems", "The document has a structural error and was not changed.");
        break;
      case "timeout":
        setStatus("origami: timed out", `origami did not answer within ${timeoutMs / 1000} seconds.`);
        break;
      case "failed":
        setStatus("origami: error – see Output", "origami could not format the document.");
        break;
    }
  }

  const mismatch = () =>
    setStatus("origami: version mismatch", "No origami binary satisfies required_version in origami.toml.");

  const previews = new PreviewManager({ resolver, log, report: reportOutcome, mismatch });
  // Enables "Preview Changes" in the command palette.
  const updateFormattable = () => {
    const doc = vscode.window.activeTextEditor?.document;
    const ok = doc !== undefined && doc.uri.scheme !== PREVIEW_SCHEME && vscode.languages.match(SELECTOR, doc) > 0;
    void vscode.commands.executeCommand("setContext", "origami.formattable", ok);
  };
  updateFormattable();

  async function run(
    document: vscode.TextDocument,
    token: vscode.CancellationToken,
    ranges: readonly vscode.Range[],
  ): Promise<vscode.TextEdit[]> {
    {
      const resolved = await resolver.resolve(document);
      if (!resolved) {
        mismatch();
        return [];
      }
      const outcome = await formatDocument(document, resolved, token, log, ranges);
      reportOutcome(document, outcome, FORMAT_TIMEOUT_MS);
      return outcome.edits;
    }
  }

  context.subscriptions.push(
    output,
    diagnostics,
    status,
    vscode.languages.registerDocumentFormattingEditProvider(SELECTOR, provider),
    vscode.languages.registerDocumentRangeFormattingEditProvider(SELECTOR, provider),
    vscode.commands.registerCommand("origami.showOutput", () => output.show()),
    previews,
    vscode.commands.registerCommand("origami.previewChanges", () => previews.previewChanges()),
    vscode.commands.registerCommand("origami.applyPreview", (uri?: vscode.Uri) =>
      previews.applyPreview(uri instanceof vscode.Uri ? uri : undefined),
    ),
    vscode.window.onDidChangeActiveTextEditor(updateFormattable),
    vscode.workspace.onDidOpenTextDocument(updateFormattable),
    vscode.workspace.onDidGrantWorkspaceTrust(() => resolver.invalidate()),
    vscode.workspace.onDidChangeConfiguration((e) => {
      if (e.affectsConfiguration("origami")) {
        resolver.invalidate();
      }
    }),
    vscode.workspace.onDidSaveTextDocument((d) => {
      if (d.fileName.endsWith("origami.toml")) {
        resolver.invalidate();
      }
    }),
    vscode.workspace.onDidCloseTextDocument((d) => diagnostics.delete(d.uri)),
  );
}

export function deactivate(): void {}
