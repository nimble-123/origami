// "origami: Preview Changes": shows the formatting of a document in VS Code's
// diff view before anything is applied. Left: a snapshot of the document when
// the command ran; right: the formatted snapshot. Both are read-only virtual
// documents of the `origami-preview` scheme.
//
// "origami: Apply Preview" applies the result. If the document changed after
// the snapshot, it formats the current text again, so later edits are never
// overwritten. The document stays unsaved.

import * as vscode from "vscode";
import * as path from "node:path";
import { BinaryResolver } from "./binary";
import { computeEdits } from "./edits";
import { formatText, TextOutcome } from "./format";
import { decode, encode, SCHEME } from "./previewUri";

/** Timeout of a preview request; longer than for format-on-save, since the
 * user waits for the diff view on purpose. */
export const PREVIEW_TIMEOUT_MS = 5000;

interface Preview {
  id: number;
  source: vscode.Uri;
  version: number;
  before: string;
  after: string;
}

/** Result of a command, returned for tests. */
export type PreviewResult = "opened" | "unchanged" | "error" | "failed" | "mismatch" | "none";
export type ApplyResult = "applied" | "unchanged" | "closed" | "failed" | "none";

export interface PreviewDeps {
  resolver: BinaryResolver;
  log: (m: string) => void;
  /** Problems panel and status bar for an outcome. */
  report: (document: vscode.TextDocument, outcome: TextOutcome, timeoutMs: number) => void;
  /** Status bar when no origami binary satisfies required_version. */
  mismatch: () => void;
}

function nameOf(uri: vscode.Uri): string {
  return path.posix.basename(uri.path) || "document";
}

function toTextEdits(before: string, after: string): vscode.TextEdit[] {
  return computeEdits(before, after).map(
    (e) =>
      new vscode.TextEdit(
        new vscode.Range(e.start.line, e.start.character, e.end.line, e.end.character),
        e.text,
      ),
  );
}

function diffInput(tab: vscode.Tab | undefined): vscode.TabInputTextDiff | undefined {
  const input = tab?.input;
  return input instanceof vscode.TabInputTextDiff && input.modified.scheme === SCHEME
    ? input
    : undefined;
}

export class PreviewManager implements vscode.TextDocumentContentProvider, vscode.Disposable {
  private readonly previews = new Map<number, Preview>();
  private nextId = 1;
  private readonly subscriptions: vscode.Disposable[];

  constructor(private readonly deps: PreviewDeps) {
    this.subscriptions = [
      vscode.workspace.registerTextDocumentContentProvider(SCHEME, this),
      vscode.window.onDidChangeActiveTextEditor(() => this.updateContext()),
      vscode.window.tabGroups.onDidChangeTabs((e) => {
        for (const tab of e.closed) {
          const input = diffInput(tab);
          const ref = input && decode(input.modified.toString());
          if (ref) {
            this.previews.delete(ref.id);
          }
        }
        this.updateContext();
      }),
    ];
  }

  dispose(): void {
    for (const s of this.subscriptions) {
      s.dispose();
    }
    this.previews.clear();
  }

  provideTextDocumentContent(uri: vscode.Uri): string {
    const ref = decode(uri.toString());
    const p = ref && this.previews.get(ref.id);
    if (!p) {
      return "";
    }
    return ref.side === "before" ? p.before : p.after;
  }

  private updateContext(): void {
    const active = diffInput(vscode.window.tabGroups.activeTabGroup.activeTab) !== undefined;
    void vscode.commands.executeCommand("setContext", "origami.previewActive", active);
  }

  private async format(document: vscode.TextDocument, text: string): Promise<TextOutcome | undefined> {
    const resolved = await this.deps.resolver.resolve(document);
    if (!resolved) {
      this.deps.mismatch();
      void vscode.window.showWarningMessage(
        "origami: no origami binary satisfies required_version in origami.toml.",
      );
      return undefined;
    }
    const cts = new vscode.CancellationTokenSource();
    try {
      const outcome = await formatText(
        text,
        document,
        resolved,
        cts.token,
        this.deps.log,
        [],
        PREVIEW_TIMEOUT_MS,
      );
      this.deps.report(document, outcome, PREVIEW_TIMEOUT_MS);
      return outcome;
    } finally {
      cts.dispose();
    }
  }

  async previewChanges(): Promise<PreviewResult> {
    const document = vscode.window.activeTextEditor?.document;
    if (!document || document.uri.scheme === SCHEME) {
      return "none";
    }
    const before = document.getText();
    const version = document.version;
    const outcome = await this.format(document, before);
    if (!outcome) {
      return "mismatch";
    }
    switch (outcome.status) {
      case "error":
        void vscode.window.showWarningMessage(
          "origami: not changed because of a structural error; see the Problems panel.",
        );
        return "error";
      case "timeout":
      case "failed":
        void vscode.window.showErrorMessage("origami: preview failed; see Output.");
        return "failed";
      case "ok":
        break;
    }
    if (outcome.text === undefined) {
      void vscode.window.showInformationMessage("origami: nothing to change.");
      return "unchanged";
    }
    const id = this.nextId++;
    this.previews.set(id, { id, source: document.uri, version, before, after: outcome.text });
    const name = nameOf(document.uri);
    const left = vscode.Uri.parse(encode({ id, side: "before", name }));
    const right = vscode.Uri.parse(encode({ id, side: "after", name }));
    await vscode.commands.executeCommand("vscode.diff", left, right, `${name}: origami preview`);
    this.updateContext();
    return "opened";
  }

  /** `uri` is the diff editor's resource when run from its title bar;
   * otherwise the active diff tab is used. */
  async applyPreview(uri?: vscode.Uri): Promise<ApplyResult> {
    const target = uri ?? diffInput(vscode.window.tabGroups.activeTabGroup.activeTab)?.modified;
    const ref = target && decode(target.toString());
    const preview = ref && this.previews.get(ref.id);
    if (!preview) {
      return "none";
    }
    const document = vscode.workspace.textDocuments.find(
      (d) => d.uri.toString() === preview.source.toString(),
    );
    if (!document || document.isClosed) {
      void vscode.window.showWarningMessage("origami: the document is closed; nothing was applied.");
      return "closed";
    }
    let edits: vscode.TextEdit[];
    if (document.version === preview.version) {
      edits = toTextEdits(preview.before, preview.after);
    } else {
      // Edited after the snapshot: format the current text instead.
      const current = document.getText();
      const version = document.version;
      const outcome = await this.format(document, current);
      if (!outcome || outcome.status !== "ok" || document.version !== version) {
        void vscode.window.showWarningMessage(
          "origami: the document could not be formatted again; nothing was applied.",
        );
        return "failed";
      }
      edits = outcome.text === undefined ? [] : toTextEdits(current, outcome.text);
    }
    let result: ApplyResult = "unchanged";
    if (edits.length > 0) {
      const we = new vscode.WorkspaceEdit();
      we.set(document.uri, edits);
      if (!(await vscode.workspace.applyEdit(we))) {
        void vscode.window.showWarningMessage("origami: the changes could not be applied.");
        return "failed";
      }
      result = "applied";
    }
    await this.closeTabs(preview.id);
    this.previews.delete(preview.id);
    return result;
  }

  private async closeTabs(id: number): Promise<void> {
    const tabs = vscode.window.tabGroups.all
      .flatMap((g) => g.tabs)
      .filter((t) => {
        const input = diffInput(t);
        return input !== undefined && decode(input.modified.toString())?.id === id;
      });
    if (tabs.length > 0) {
      await vscode.window.tabGroups.close(tabs);
    }
  }
}
