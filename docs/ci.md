# origami in CI

All variants download a released binary and run `origami --check`: exit 0 if
everything is formatted, 1 if a file would change, 2 for a structural error
(see [CLI.md](CLI.md)). Replace `<origami repository URL>` and the version.
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
      - uses: <owner>/origami@v1.0.0
        with:
          args: --check --changed origin/${{ github.base_ref }}
          sarif: "true"        # findings as code-scanning annotations
```

Without `version`, the action downloads the release that matches the tag it
is used at (`@v1.0.0`), else the latest release.

## GitLab CI

```yaml
origami:
  image: debian:stable-slim
  variables:
    ORIGAMI_VERSION: "1.0.0"
    GIT_DEPTH: 0
  before_script:
    - apt-get update -qq && apt-get install -y -qq curl ca-certificates git
    - curl -fsSL -o origami "<origami repository URL>/releases/download/v${ORIGAMI_VERSION}/origami-${ORIGAMI_VERSION}-x86_64-unknown-linux-musl"
    - curl -fsSL -o origami.sha256 "<origami repository URL>/releases/download/v${ORIGAMI_VERSION}/origami-${ORIGAMI_VERSION}-x86_64-unknown-linux-musl.sha256"
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
  ORIGAMI_VERSION: "1.0.0"

steps:
  - checkout: self
    fetchDepth: 0
  - script: |
      set -e
      base="<origami repository URL>/releases/download/v$(ORIGAMI_VERSION)"
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

The JSON diagnostics (`--diagnostics json`, on stderr) follow
[schemas/diagnostics.v1.json](../schemas/diagnostics.v1.json);
`--diagnostics sarif` produces SARIF 2.1.0 for tools that read it.
