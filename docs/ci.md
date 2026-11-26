# origami in CI

<!-- x-release-please-start-version -->

All variants download a released binary and run `origami --check`: exit 0 if
everything is formatted, 1 if a file would change, 2 for a structural error
(see [CLI.md](CLI.md)). The version below is the current release.
Each release ships a `.sha256` file next to every binary.

To introduce origami into an existing code base without reformatting
everything, check only what a change touches: `--changed <base revision>`.

## GitHub Actions

```yaml
name: origami
on: [pull_request]
permissions:
  contents: read
  security-events: write   # only for sarif: "true"
jobs:
  format:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0       # only for --changed
      - uses: nimble-123/origami@v0.4.2
        with:
          args: --check --changed origin/${{ github.base_ref }}
          sarif: "true"        # findings as code-scanning annotations
```

Without `version`, the action downloads the release that matches the tag it
is used at (`@v0.4.2`), else the latest release. When the action is pinned by
commit SHA or used at a branch or another floating ref, the binary version is
not pinned with it: the action prints a warning (it does not fail) and
installs the newest release. Set `version: v0.4.2` to pin the binary as well.

"The newest release" means the newest one that has a binary and a `.sha256`
file for the runner: release-please creates a release when it tags, and the
binaries are attached some time later, so the action skips a release that has
none yet. It fails, with a message, if none of the 30 newest releases has one.

The action checks the checksum of the download and its build provenance
attestation. Releases made before attestations existed have none; for them it
prints a warning and checks the checksum only, unless `require-attestation:
"true"` is set, which refuses them. Only "there is no attestation" (HTTP 404 or
an empty list) counts as none: any other error while looking it up (rate limit,
server error, a token without access) fails the step, as does a binary whose
attestation does not verify.

## GitLab CI

```yaml
origami:
  image: debian:stable-slim
  variables:
    ORIGAMI_VERSION: "0.4.2"
    GIT_DEPTH: 0
  before_script:
    - apt-get update -qq && apt-get install -y -qq curl ca-certificates git
    - curl -fsSL -o origami "https://github.com/nimble-123/origami/releases/download/v${ORIGAMI_VERSION}/origami-${ORIGAMI_VERSION}-x86_64-unknown-linux-musl"
    - curl -fsSL -o origami.sha256 "https://github.com/nimble-123/origami/releases/download/v${ORIGAMI_VERSION}/origami-${ORIGAMI_VERSION}-x86_64-unknown-linux-musl.sha256"
    - echo "$(cut -d' ' -f1 origami.sha256)  origami" | sha256sum -c -
    - chmod +x origami
  script:
    # In merge requests: only the lines the merge request changes.
    - if [ -n "$CI_MERGE_REQUEST_TARGET_BRANCH_NAME" ]; then
        git fetch -q origin "$CI_MERGE_REQUEST_TARGET_BRANCH_NAME";
        ./origami --check --diff --changed "origin/$CI_MERGE_REQUEST_TARGET_BRANCH_NAME";
      else
        ./origami --check;
      fi
  rules:
    - if: $CI_PIPELINE_SOURCE == "merge_request_event"
    - if: $CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH
```

## Azure Pipelines

```yaml
variables:
  ORIGAMI_VERSION: "0.4.2"

steps:
  - checkout: self
    fetchDepth: 0
  - script: |
      set -e
      base="https://github.com/nimble-123/origami/releases/download/v$(ORIGAMI_VERSION)"
      name="origami-$(ORIGAMI_VERSION)-x86_64-unknown-linux-musl"
      curl -fsSL -o origami "$base/$name"
      curl -fsSL -o origami.sha256 "$base/$name.sha256"
      echo "$(cut -d' ' -f1 origami.sha256)  origami" | sha256sum -c -
      chmod +x origami
      ./origami --check --diff
    displayName: origami
```

## Any other CI, or without released binaries

```sh
cargo install --locked abap-origami   # once v1.0 is on crates.io
origami --check
```

<!-- x-release-please-end -->

The JSON diagnostics (`--diagnostics json`, on stderr) follow
[schemas/diagnostics.v1.json](../schemas/diagnostics.v1.json);
`--diagnostics sarif` produces SARIF 2.1.0 for tools that read it.
