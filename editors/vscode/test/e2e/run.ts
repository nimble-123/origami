// Starts a downloaded VS Code with this extension and a temporary workspace,
// then runs test/e2e/suite.ts inside the extension host.

import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { runTests } from "@vscode/test-electron";

async function main(): Promise<void> {
  const root = path.resolve(__dirname, "../../..");
  const workspace = fs.mkdtempSync(path.join(os.tmpdir(), "origami-e2e-"));
  // workspaceContains activation is evaluated at startup, as in a real
  // abapGit repository.
  fs.writeFileSync(path.join(workspace, "zseed.prog.abap"), "WRITE 'seed'.\n");
  try {
    await runTests({
      extensionDevelopmentPath: root,
      extensionTestsPath: path.resolve(__dirname, "suite.js"),
      launchArgs: [workspace, "--disable-extensions", "--disable-workspace-trust"],
      extensionTestsEnv: { ORIGAMI_E2E_WORKSPACE: workspace },
    });
  } finally {
    fs.rmSync(workspace, { recursive: true, force: true });
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
