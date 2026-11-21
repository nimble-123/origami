# Third-party notices

## Rust dependencies

This section is generated for each release by
`cargo about generate about.hbs > THIRD_PARTY_NOTICES.generated.md` and
appended here by the release workflow.

## References consulted

origami is an independent implementation. These projects were consulted as
references for statement coverage, test cases and layout conventions:

| Project | Version consulted | License |
|---|---|---|
| abaplint (`@abaplint/core`) | 2.120.112 | MIT |
| ABAP Cleaner | 1.29.2 | Apache-2.0 |

No source code was copied. Test inputs taken from abaplint are listed below
with their license. (None so far.)

## Test corpus

The corpus in `corpus/manifest.toml` is fetched at test time and never
redistributed. Each entry records its SPDX license.
