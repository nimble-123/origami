# Security policy

## Supported versions

Only the latest 0.x release receives security fixes. origami is pre-1.0; there
are no maintenance branches for older releases. Check your version with
`origami --version`.

## Reporting a vulnerability

Please do not open a public issue for a vulnerability. Report it privately
with GitHub's private vulnerability reporting:

1. Open the [Security tab](https://github.com/nimble-123/origami/security) of
   the repository.
2. Choose **Report a vulnerability** and describe the problem: the origami
   version, the command or editor action, and an input that triggers it.

Include only the lines of source that show the problem; strip anything
confidential.

The maintainer replies in the same thread. origami is maintained by one
person, so there is no guaranteed response time. A confirmed vulnerability is
fixed in a new release and published as a GitHub security advisory.

## What counts

origami reads source files, writes them back, and runs as a command-line tool,
a VS Code extension, an MCP server and a WebAssembly build. Examples worth a
private report:

* an input that makes origami write outside the files it was given, or run
  something it should not;
* an input that makes origami change the meaning of the code (the self-check
  should prevent this; see [docs/INVARIANTS.md](docs/INVARIANTS.md));
* a flaw in the release artifacts, the GitHub Action or the VS Code extension
  that lets a third party run code on a user's machine.

A crash, a hang or an idempotency failure on odd input is a bug, not a
vulnerability: use the
[bug report](https://github.com/nimble-123/origami/issues/new?template=bug_report.yml).
