# Releasing origami

A release is a version tag `vX.Y.Z`. [release-please](https://github.com/googleapis/release-please)
creates it when its release pull request is merged; the tag starts
`.github/workflows/release.yml`. Everything that needs a secret is skipped
when the secret is missing, so the workflow works without any of them and
produces unsigned binaries, checksums, an SBOM and attestations only. This page
lists what to create to turn the other steps on.

## What the release workflow does

| Job | Result |
|---|---|
| `build` | Binaries for 7 targets (macOS x64/arm64, Windows x64/arm64, Linux gnu x64/arm64, Linux musl x64). Signed and notarized (macOS) or signed (Windows) when the credentials exist. |
| `smoke` | Starts every binary on a runner of its own platform, prints `--version`, formats a fixture. A binary that does not run stops the release. |
| `vsix` | One VS Code extension package per target, with the binary inside. |
| `extras` | Third-party notices, the CycloneDX SBOM (`origami-X.Y.Z.sbom.cdx.json`), and `release-diff.patch` (what the new release formats differently from the previous one, on the test corpus). |
| `release` | Attests build provenance for the binaries, the SBOM and the extension packages, then attaches everything to the GitHub release. |
| `homebrew`, `crates`, `vscode-publish` | Formula in the tap, crates.io, VS Code Marketplace and Open VSX. |

Secrets are passed only to the step that uses them. The build steps and the
dependency build scripts never see them.

## Secrets and accounts

Set repository secrets at *Settings > Secrets and variables > Actions*, or with
`gh secret set NAME --repo nimble-123/origami`. Nothing below is stored in the
repository.

| Secret | Needed for | If missing |
|---|---|---|
| `RELEASE_PLEASE_TOKEN` | release-please, so that the tag it pushes starts `release.yml` | the default token is used; the release pull request works, but **the tag does not start the release workflow** |
| `APPLE_CERT_P12`, `APPLE_CERT_PASSWORD`, `APPLE_API_KEY`, `APPLE_API_KEY_ID`, `APPLE_API_ISSUER` | macOS signing and notarization (all five) | macOS binaries are not signed |
| `WINDOWS_CERT_PFX`, `WINDOWS_CERT_PASSWORD` | Windows signing (both) | Windows binaries are not signed |
| `CARGO_REGISTRY_TOKEN` | publishing to crates.io | crates are checked with a dry run but not published |
| `OVSX_PAT` | Open VSX, namespace `nimble-123` | not published there |
| `HOMEBREW_TAP_TOKEN` | the Homebrew formula in `nimble-123/homebrew-tap` | formula not updated |

### RELEASE_PLEASE_TOKEN

1. Create a [fine-grained personal access token](https://github.com/settings/personal-access-tokens/new)
   for the repository `nimble-123/origami` only, with *Contents: Read and
   write* and *Pull requests: Read and write*.
2. Save it as `RELEASE_PLEASE_TOKEN`.

### macOS: signing and notarization

You need an [Apple Developer Program](https://developer.apple.com/programs/enroll/)
membership (paid, per year).

1. **Certificate.** In [Certificates, Identifiers & Profiles](https://developer.apple.com/account/resources/certificates/list)
   create a *Developer ID Application* certificate (needs the account holder
   or admin role). Create the signing request with Keychain Access (*Certificate
   Assistant > Request a Certificate from a Certificate Authority*, saved to
   disk), upload it, download the `.cer` and open it.
2. **Export.** In Keychain Access, select the certificate together with its
   private key (*My Certificates*) and export a `.p12` with a password.
   - `APPLE_CERT_P12`: `base64 -i cert.p12 | gh secret set APPLE_CERT_P12 --repo nimble-123/origami`
   - `APPLE_CERT_PASSWORD`: the password you chose.
3. **Notarization key.** In [App Store Connect > Users and Access > Integrations > App Store Connect API](https://appstoreconnect.apple.com/access/integrations/api)
   create a team key with the *Developer* role and download the `.p8` file
   (it can be downloaded once).
   - `APPLE_API_KEY`: the full text of the `.p8` file.
   - `APPLE_API_KEY_ID`: the key ID shown in the table.
   - `APPLE_API_ISSUER`: the *Issuer ID* shown above the table.

Details: [Notarizing macOS software before distribution](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution).
A plain executable cannot be stapled, so Gatekeeper checks the notarization
ticket online the first time a downloaded binary runs.

### Windows: code signing

The workflow signs with `signtool` and a `.pfx` file from the secrets
(`WINDOWS_CERT_PFX` is the base64 of the file, `WINDOWS_CERT_PASSWORD` its
password) and verifies the signature afterwards.

Be aware that code-signing certificates issued since June 2023 must keep their
private key on a hardware token or in a cloud service, and cannot be exported
as a `.pfx`. A `.pfx` therefore only works with a certificate from before that
date or from a provider that still allows an exportable key. For a new
certificate, the practical routes are [Azure Artifact Signing](https://learn.microsoft.com/azure/artifact-signing/)
(formerly Trusted Signing), SSL.com eSigner or DigiCert KeyLocker. Each needs
a different signing step in `release.yml` instead of the `.pfx` one; the rest
of the workflow stays as it is. Until then, leave the two secrets unset:
Windows binaries ship unsigned, with a checksum and a provenance attestation.

### crates.io: CARGO_REGISTRY_TOKEN

1. Log in at [crates.io](https://crates.io) with GitHub and verify your email.
2. Create a token at [crates.io/settings/tokens](https://crates.io/settings/tokens)
   with the scopes *publish-new* (the first release creates the crates) and
   *publish-update*, restricted to the crate patterns `origami_*` and
   `abap-origami`.
3. Save it as `CARGO_REGISTRY_TOKEN`.

The six crate names are free at the time of writing. The workflow runs
`cargo publish --workspace --dry-run` on every release, with or without the
token, so manifest problems show up before the token exists.

### VS Code Marketplace: trusted publishing (no secret)

The extension is published as `nimble-123.abap-origami` with OIDC trusted
publishing: the Marketplace trusts the `vscode-publish` job of `release.yml`,
so no Marketplace token is stored. The job runs in the GitHub environment
`vscode-marketplace` and calls `vsce publish --oidc`.

1. Create the publisher `nimble-123` at
   [marketplace.visualstudio.com/manage](https://marketplace.visualstudio.com/manage/createpublisher).
2. On the publisher's *Trusted Publishing* tab, create a policy:

   | Field | Value |
   | --- | --- |
   | Policy name | `origami release` |
   | Provider | GitHub Actions |
   | Repository owner | `nimble-123` |
   | Repository | `origami` |
   | Workflow | `release.yml` |
   | Environment | `vscode-marketplace` |

   See [Publish with OIDC trusted publishing](https://learn.microsoft.com/en-us/azure/devops/extend/publish/overview?view=azure-devops#publish-with-oidc-trusted-publishing).
3. Optional: in the repository settings, give the `vscode-marketplace`
   environment a deployment branch rule for release tags (`v*`) or required
   reviewers. GitHub creates the environment on the first run if it does not
   exist.

### Open VSX: OVSX_PAT

1. Log in at [open-vsx.org](https://open-vsx.org) with GitHub and sign the
   Eclipse Foundation publisher agreement in your profile.
2. Create an access token at [open-vsx.org/user-settings/tokens](https://open-vsx.org/user-settings/tokens)
   and save it as `OVSX_PAT`.
3. Create the namespace once: `npx ovsx@1.4.0 create-namespace nimble-123 -p <token>`.
   See [Publishing Extensions](https://github.com/eclipse/openvsx/wiki/Publishing-Extensions).

### Homebrew tap: HOMEBREW_TAP_TOKEN

1. Create the repository `nimble-123/homebrew-tap` (public, with a first
   commit).
2. Create a [fine-grained personal access token](https://github.com/settings/personal-access-tokens/new)
   for that repository only, with *Contents: Read and write*.
3. Save it as `HOMEBREW_TAP_TOKEN`. Users then run
   `brew install nimble-123/tap/origami`.

### Repository settings

* *Settings > Pages > Source*: GitHub Actions (for the website workflow).
* Build provenance attestations need a public repository.
* Optional hardening: put the secrets in a GitHub *environment* with required
  reviewers, and add `environment: <name>` to the signing and publishing jobs,
  so that every release waits for an approval.

## Cutting a release

1. Merge pull requests with Conventional Commit titles
   (see [CONTRIBUTING.md](../CONTRIBUTING.md)). release-please opens or
   updates the pull request *chore: release X.Y.Z* with the version bump,
   `CHANGELOG.md` and `Cargo.lock`.
2. Read the changelog in that pull request.
3. Merge it. release-please tags `vX.Y.Z` and creates the GitHub release with
   the changelog; the tag starts the *Release* workflow.
4. Watch the run (`gh run watch`). The `smoke` jobs must be green before
   anything is attached; a red `smoke` job means a binary does not start on
   its platform.
5. Check the result:
   - the release has 7 binaries with `.sha256` files, 7 `.vsix` files,
     `THIRD_PARTY_NOTICES.md`, the SBOM and, when an earlier release has
     binaries, `release-diff.patch` (look at it: it lists every formatting
     change of the release on the corpus);
   - `gh attestation verify origami-X.Y.Z-<target> --repo nimble-123/origami`
     succeeds;
   - macOS: `spctl --assess --type execute -v origami-X.Y.Z-aarch64-apple-darwin`
     and `codesign -dv` on a downloaded binary, when signing is on;
   - the crate pages, the Marketplace page and the tap formula show the new
     version, when those secrets are set.

A failed job can be re-run with *Re-run failed jobs*; the release assets are
uploaded with `--clobber`. crates.io and the Marketplace refuse a version that
exists already, so after a partial publish continue by hand with the missing
registry rather than re-running the publishing job.

## Verifying a download

```sh
gh attestation verify origami-X.Y.Z-x86_64-unknown-linux-gnu \
  --repo nimble-123/origami \
  --signer-workflow nimble-123/origami/.github/workflows/release.yml
sha256sum -c origami-X.Y.Z-x86_64-unknown-linux-gnu.sha256
```

The GitHub Action (`action.yml`) does both checks. For releases made before
attestations existed it prints a warning and checks the checksum only; set
`require-attestation: true` to refuse those.

## Pinned versions

Actions are pinned to full commit SHAs with the version in a comment, and the
build tools to exact versions (`cargo-zigbuild`, `ziglang`, `cargo-about`,
`cargo-cyclonedx`, `vsce`, `ovsx`). Dependabot (`.github/dependabot.yml`) opens
a grouped weekly pull request per ecosystem, including the SHAs of the actions.
The `ziglang`, `cargo-zigbuild`, `ovsx` and `vsce` versions in `release.yml`
are not covered by Dependabot; update them by hand when a release build asks
for it.
