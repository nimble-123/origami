# Changelog

## [0.1.1](https://github.com/nimble-123/origami/compare/v0.1.0...v0.1.1) (2026-11-23)


### Fixes

* **abap:** keep the rest of a VALUE row after a parameter keyword ([#4](https://github.com/nimble-123/origami/issues/4)) ([05fc99d](https://github.com/nimble-123/origami/commit/05fc99d5b14032c8682faf19d7b72ae4361c2f18))


### Formatting output

* **abap:** upper-case keywords wherever they stand (ABAP-CASE-001) ([#9](https://github.com/nimble-123/origami/issues/9)) ([0074466](https://github.com/nimble-123/origami/commit/0074466ff9958ceadd729000aee3852e3f081eee))
* **abap:** write identifiers without lower-case letters in lower case (ABAP-CASE-002) ([#10](https://github.com/nimble-123/origami/issues/10)) ([b7850e5](https://github.com/nimble-123/origami/commit/b7850e51fe380ca16c0f19673a6d5d92b180af01))

## 0.1.0 (2026-11-22)


### Features

* **abap:** CALL FUNCTION and CALL METHOD groups, chains, pragmas, more statements ([ff5e470](https://github.com/nimble-123/origami/commit/ff5e470d40a79da4e17ec80206aea59d54ff7bb2))
* **abap:** chains of keyword statements and more statements ([dd7a841](https://github.com/nimble-123/origami/commit/dd7a841cb9b624f17299bec97fc8e70f2d4e70ed))
* **abap:** classic ABAP grammar for reports, events, FORM routines and function modules ([27d20bb](https://github.com/nimble-123/origami/commit/27d20bbd984407732ebb2eedf486b363168ae292))
* **abap:** EML layout (ABAP-EML-001) ([5677791](https://github.com/nimble-123/origami/commit/567779142da76cc8de91845eafd33170b316bea2))
* **abap:** grammar up to SAP_BASIS 8.16, checked against the abaplint catalog ([9584a1c](https://github.com/nimble-123/origami/commit/9584a1ce7376ac3420310da0eaa39692fca7dafa))
* **abap:** printer, statement-level layout and ABAP style batch 1 ([e7b8c06](https://github.com/nimble-123/origami/commit/e7b8c067663661d0599809a88d7c23797eefbf19))
* **cds:** case elements, DCL grants, CLI benchmark and release workflow ([a32d2a1](https://github.com/nimble-123/origami/commit/a32d2a16e08d1822c56660d2e368a45e83280d12))
* **cds:** grammar-based annotations and definition headers ([87f9170](https://github.com/nimble-123/origami/commit/87f91707410ba53955b7db761a8a46955684c65e))
* **cli:** origami --review with notes on introduced line breaks ([d53303b](https://github.com/nimble-123/origami/commit/d53303b741488dd1460b47d315a2ac7f5073a2f4))
* complete ABAP layout (M1) and structural layout for the CDS family ([af228d3](https://github.com/nimble-123/origami/commit/af228d3433195368918e3186a12a30b956eb3e7f))
* **core:** explain infrastructure for introduced line breaks ([66817f0](https://github.com/nimble-123/origami/commit/66817f0c07ffd080a4dbcbba8cdd9558f4ad1cdc))
* **core:** rule spans and note sentences for line breaks ([7fe2e50](https://github.com/nimble-123/origami/commit/7fe2e5043eb9def8a912585de4cea820a9b9187a))
* partial formatting, MCP server, dynpro flow logic, CI action and new output formats ([bb11104](https://github.com/nimble-123/origami/commit/bb1110489bb5da52eef6514d3700f3b9c786ad44))
* **vscode:** Preview Changes and Apply Preview ([5ef69fc](https://github.com/nimble-123/origami/commit/5ef69fc7e5c04a6e92a63b738f4271854655841c))
* **vscode:** VS Code extension for files and ADT documents ([3a57fdf](https://github.com/nimble-123/origami/commit/3a57fdfe07e42612128c6a9da0403027ec3d9c37))
* walking skeleton for the origami ABAP and CDS formatter (M0) ([061a364](https://github.com/nimble-123/origami/commit/061a36448aa8cf670c49e7d4fc89fa825d3037bf))
* website, playground, brand, Homebrew and release automation ([d3791a7](https://github.com/nimble-123/origami/commit/d3791a7342417085241cdc7501bef6fc4691aa01))


### Fixes

* **abap:** database DML breaks before every clause (ABAP-SQL-001) ([ecfe2b1](https://github.com/nimble-123/origami/commit/ecfe2b11606414db0359ed6fd74b9062e1dc187c))
* **abap:** fuzzing fixes, ASSIGN keyword case, ABAP Cleaner output comparison ([5e399ab](https://github.com/nimble-123/origami/commit/5e399aba7bc877f630e66ceca3a2b22878587721))
* **abap:** upper-case SINGLE and DISTINCT in SELECT ... FROM ... FIELDS (ABAP-SQL-001) ([156b4a7](https://github.com/nimble-123/origami/commit/156b4a7855ade75a6f464d9e91b517e28cc3504b))
* **layout:** comment after the period and fill width (ABAP-COMMENT-002, ABAP-WIDTH-001) ([574fd3b](https://github.com/nimble-123/origami/commit/574fd3b10e030811d048b537fcbca59861c83255))
* **mcp:** format_files stays below the working directory ([49b213a](https://github.com/nimble-123/origami/commit/49b213a88f6acc2a48193dc0e7e1804d875bcd2c))
* robustness fixes from longer fuzzing runs ([22c8ac2](https://github.com/nimble-123/origami/commit/22c8ac2399ff2f26e936b14351bd8d5aa94317ae))
* six layout fixes found by a mutation stress test ([537fbe5](https://github.com/nimble-123/origami/commit/537fbe536ebcd216dac0a3fb54317ed98cc47c6b))


### Performance

* benchmark matrix with per-file percentiles, cold and warm starts, agents and threads ([cdaeea7](https://github.com/nimble-123/origami/commit/cdaeea7e664859b43c6a5708ca3407e12845f5c6))
* faster formatting, printer tests and an adoption guide ([17ec570](https://github.com/nimble-123/origami/commit/17ec5706ba6f6ac0161450453bec9704f2440677))


### Formatting output

* **abap:** keep SELECT FROM dbtab on the first line in the strict syntax (ABAP-SQL-001) ([9f90de7](https://github.com/nimble-123/origami/commit/9f90de796fd037aa72bb367837f7e88b8c6bd401))

## Changelog

All notable changes to origami are documented here. From 0.1.0 on,
[release-please](https://github.com/googleapis/release-please) writes this file
from [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/)
(see [CONTRIBUTING.md](CONTRIBUTING.md)). Within a style edition, every output
change names its STYLE rule ID in the commit message. Detailed notes on the
work before 0.1.0: [docs/development-history.md](docs/development-history.md).
