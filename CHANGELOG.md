# Changelog

## [0.4.2](https://github.com/nimble-123/origami/compare/v0.4.1...v0.4.2) (2026-11-26)


### Fixes

* **abap:** recognize CATCH SYSTEM-EXCEPTIONS and ENDCATCH ([#97](https://github.com/nimble-123/origami/issues/97)) ([3cb63d4](https://github.com/nimble-123/origami/commit/3cb63d4d5b8bcd77417d8c396c3a71ee8b4e7fb9))
* **abap:** upper-case RAP handler parameter keywords in aligned signatures ([#87](https://github.com/nimble-123/origami/issues/87)) ([9a5493e](https://github.com/nimble-123/origami/commit/9a5493e4efd59aab9c359d96f2a13110bd85057a))


### Formatting output

* **abap:** EML clauses on their own lines (ABAP-EML-001) ([#95](https://github.com/nimble-123/origami/issues/95)) ([6b65cfd](https://github.com/nimble-123/origami/commit/6b65cfd1dae9f934f10465baa724db28a34efa63))
* **abap:** iterations in constructor expressions (ABAP-VALUE-002) ([#94](https://github.com/nimble-123/origami/issues/94)) ([3912b04](https://github.com/nimble-123/origami/commit/3912b0496624fb15d679c5f12be16902682520a8))
* **abap:** key components after WITH KEY one per line (ABAP-ALIGN-010) ([#99](https://github.com/nimble-123/origami/issues/99)) ([6e2b5f3](https://github.com/nimble-123/origami/commit/6e2b5f31f850f1df79d635c341964501c8d09765))
* **abap:** method signature tables below the method name (ABAP-METHODS-001) ([#91](https://github.com/nimble-123/origami/issues/91)) ([ba318cf](https://github.com/nimble-123/origami/commit/ba318cfb983e7d145919bdcf238a5370783f9674))
* **abap:** one item per line in CLEAR:, FREE:, SORT and CATCH (ABAP-ALIGN-009) ([#90](https://github.com/nimble-123/origami/issues/90)) ([33c3156](https://github.com/nimble-123/origami/commit/33c3156f241739641f4fe87c0039a3e50ac7d07a))
* **abap:** structures align as ABAP Cleaner (ABAP-ALIGN-003, ABAP-DECL-002) ([#92](https://github.com/nimble-123/origami/issues/92)) ([7a52274](https://github.com/nimble-123/origami/commit/7a5227431ce143477f30198d04f97ac3f1c9af66))

## [0.4.1](https://github.com/nimble-123/origami/compare/v0.4.0...v0.4.1) (2026-11-25)


### Fixes

* **cds:** leave lists with braces as written ([#85](https://github.com/nimble-123/origami/issues/85)) ([136855f](https://github.com/nimble-123/origami/commit/136855f8678a62e1cbdd50962c0f6e9604c7af8e))
* **cds:** no blank lines around an element laid out as a header ([#74](https://github.com/nimble-123/origami/issues/74)) ([681460a](https://github.com/nimble-123/origami/commit/681460a3fbc3b52f668bdabe6b98cba459f742d4))
* **cds:** split a one-line block after a case in its unit ([#82](https://github.com/nimble-123/origami/issues/82)) ([2c3705a](https://github.com/nimble-123/origami/commit/2c3705a82d534de88adf1d88661eda0b8f900aa7))
* **cds:** split a one-line block after an end-of-line comment in its unit ([#77](https://github.com/nimble-123/origami/issues/77)) ([9ef845c](https://github.com/nimble-123/origami/commit/9ef845ca7a1eeca870c1150be115cbffa17721b1))


### Formatting output

* **cds:** function parameters after =&gt; one per line (CDS-ALIGN-003) ([#83](https://github.com/nimble-123/origami/issues/83)) ([9302be8](https://github.com/nimble-123/origami/commit/9302be8bd129446ab77067d9607352733d3455f9))
* **cds:** one field per line in group by and define view name lists (CDS-ALIGN-004) ([#80](https://github.com/nimble-123/origami/issues/80)) ([dfa256a](https://github.com/nimble-123/origami/commit/dfa256a8c1468cafb0e3d7d0c087d3121a1a4bf1))
* **cds:** parameters of a data source one per line (CDS-ALIGN-002) ([#81](https://github.com/nimble-123/origami/issues/81)) ([ff4cc2d](https://github.com/nimble-123/origami/commit/ff4cc2d379b829149c32d6a70f044caf4bf92ab4))

## [0.4.0](https://github.com/nimble-123/origami/compare/v0.3.1...v0.4.0) (2026-11-25)


### Features

* add the style option (aligned, compact) ([#60](https://github.com/nimble-123/origami/issues/60)) ([f96a7ea](https://github.com/nimble-123/origami/commit/f96a7ea764d204836d4a835c4f454989928f6b1d))


### Fixes

* **cds:** no alias padding for elements laid out as a header ([#71](https://github.com/nimble-123/origami/issues/71)) ([56aaf72](https://github.com/nimble-123/origami/commit/56aaf729ce8f8064fc471d39fa59bb323e5759f2))


### Formatting output

* **abap:** add blank lines between sections, groups and classes (ABAP-BLANK-002) ([#68](https://github.com/nimble-123/origami/issues/68)) ([6b891a0](https://github.com/nimble-123/origami/commit/6b891a020df813c07351c923ea32f1da3e769399))
* **abap:** align declarations in columns (ABAP-ALIGN-008, ABAP-DECL-001) ([#66](https://github.com/nimble-123/origami/issues/66)) ([f914f5f](https://github.com/nimble-123/origami/commit/f914f5f2858e8f03a7ae2dac711b13e5857c5fdf))
* **abap:** align parameter keywords and = across groups (ABAP-CALL-002) ([#63](https://github.com/nimble-123/origami/issues/63)) ([bf6f1e5](https://github.com/nimble-123/origami/commit/bf6f1e54d7730567b048cd0c3ce6af17f9818d37))
* **abap:** blank line after the declarations at the start of a file (ABAP-BLANK-002) ([#70](https://github.com/nimble-123/origami/issues/70)) ([5043104](https://github.com/nimble-123/origami/commit/504310440dfcece76600e2812bb21ac085b6f8a8))
* **abap:** CALL FUNCTION groups as one aligned table (ABAP-CALL-003) ([#64](https://github.com/nimble-123/origami/issues/64)) ([47d3f1c](https://github.com/nimble-123/origami/commit/47d3f1c8b87019c0e8a30b536c1a0f2354009c4b))
* **abap:** comment spacing and blank lines after comments (ABAP-COMMENT-003) ([#67](https://github.com/nimble-123/origami/issues/67)) ([5be0770](https://github.com/nimble-123/origami/commit/5be0770f64abb6e4f2af44024745f06c6dd3a9a6))
* **abap:** indent continuation lines by one level (ABAP-INDENT-001) ([#62](https://github.com/nimble-123/origami/issues/62)) ([cd21c27](https://github.com/nimble-123/origami/commit/cd21c279a4eeb1eb8e374838651d7a7687468832))
* **abap:** method signatures as tables behind the name (ABAP-METHODS-001) ([#65](https://github.com/nimble-123/origami/issues/65)) ([d3f2258](https://github.com/nimble-123/origami/commit/d3f22588ebd19df01f78443f5879bc7e98ebfd32))
* **cds:** aligned aliases and blank lines between view sections (CDS-ALIGN-001, CDS-BLANK-002) ([#69](https://github.com/nimble-123/origami/issues/69)) ([8f42d5e](https://github.com/nimble-123/origami/commit/8f42d5e517e5fb81d125183ed649f76c5c2c11df))

## [0.3.1](https://github.com/nimble-123/origami/compare/v0.3.0...v0.3.1) (2026-11-24)


### Fixes

* **cds:** keep line breaks of glued units inside a block ([#55](https://github.com/nimble-123/origami/issues/55)) ([b66f86a](https://github.com/nimble-123/origami/commit/b66f86a9f2a4a30ebdae814c6f7c82cbc482d904))
* **cds:** split a one-line block that holds a comment ([#57](https://github.com/nimble-123/origami/issues/57)) ([b2d09e6](https://github.com/nimble-123/origami/commit/b2d09e6665c8e6f12f1eed7f3b12ccd0fe029e05))
* two nightly fuzz findings (glued period after a comment, where inside braces) ([#52](https://github.com/nimble-123/origami/issues/52)) ([2131d11](https://github.com/nimble-123/origami/commit/2131d11457904d80f5a610f840b98b53b616685f))

## [0.3.0](https://github.com/nimble-123/origami/compare/v0.2.3...v0.3.0) (2026-11-24)


### Features

* replace the logo with a polygon crane ([#47](https://github.com/nimble-123/origami/issues/47)) ([362a5d7](https://github.com/nimble-123/origami/commit/362a5d715adc5b8feb329c4fcd19030d4eab481c))
* **site:** add the crane photos, a larger app icon and the README lockup ([#49](https://github.com/nimble-123/origami/issues/49)) ([fa6c0c8](https://github.com/nimble-123/origami/commit/fa6c0c843986df08692692890bb9265f3ccab4e0))

## [0.2.3](https://github.com/nimble-123/origami/compare/v0.2.2...v0.2.3) (2026-11-23)


### Fixes

* **release:** expect no blank before the period in the smoke fixture ([#45](https://github.com/nimble-123/origami/issues/45)) ([81bb609](https://github.com/nimble-123/origami/commit/81bb6093d141e35a7f55a53fd96014715ffa6787))

## [0.2.2](https://github.com/nimble-123/origami/compare/v0.2.1...v0.2.2) (2026-11-23)


### Fixes

* **abap:** upper-case NULLS FIRST and NULLS LAST in ORDER BY ([#40](https://github.com/nimble-123/origami/issues/40)) ([65651bc](https://github.com/nimble-123/origami/commit/65651bc6887a25f45131fdad68a679f0c7da16d4))


### Formatting output

* **abap:** align COND/SWITCH expressions and SELECT field lists (ABAP-COND-002, ABAP-SQL-003) ([#37](https://github.com/nimble-123/origami/issues/37)) ([84f0095](https://github.com/nimble-123/origami/commit/84f0095307a5768d78ca3631cb00e11392b41bb9))
* **abap:** align consecutive assignments, test methods, redefinitions and aliases (ABAP-ALIGN-004..007) ([#38](https://github.com/nimble-123/origami/issues/38)) ([7d27bfd](https://github.com/nimble-123/origami/commit/7d27bfdadb047c7ca3d87f79d29b14f419262473))
* **abap:** no space before a period or comma, and aligned ABAP Doc parameters (ABAP-SPACE-001, ABAP-DOC-001) ([#36](https://github.com/nimble-123/origami/issues/36)) ([8ef522f](https://github.com/nimble-123/origami/commit/8ef522f7f736c51373c7f68f0b47a9e4142a18ea))
* **cds:** break before WHERE, GROUP BY, HAVING and UNION in queries (CDS-CLAUSE-001) ([#34](https://github.com/nimble-123/origami/issues/34)) ([8e046ac](https://github.com/nimble-123/origami/commit/8e046ac21ca6aad969baa8f9bad3a285859cf3cf))

## [0.2.1](https://github.com/nimble-123/origami/compare/v0.2.0...v0.2.1) (2026-11-23)


### Fixes

* publish a release whose test-corpus output did not change ([#31](https://github.com/nimble-123/origami/issues/31)) ([98e4b4f](https://github.com/nimble-123/origami/commit/98e4b4f903512cf511a751c1674f6ee648d58e9b))

## [0.2.0](https://github.com/nimble-123/origami/compare/v0.1.1...v0.2.0) (2026-11-23)


### Features

* **site:** run the playground in a Web Worker; test wasm and the extension in CI ([#17](https://github.com/nimble-123/origami/issues/17)) ([9cda61c](https://github.com/nimble-123/origami/commit/9cda61c435c56f6a678cae2008ff87f755c2cdda))


### Fixes

* bound memory and time on deeply nested and huge input (W004) ([#27](https://github.com/nimble-123/origami/issues/27)) ([92086f8](https://github.com/nimble-123/origami/commit/92086f895ab12daed138d4b1222eaa7043449002))
* **cli:** harden the CLI and the MCP server ([#21](https://github.com/nimble-123/origami/issues/21)) ([cbe2106](https://github.com/nimble-123/origami/commit/cbe21063c5cbd6e627f210781a22edf5a703d447))
* idempotency at every setting, and a working nightly fuzz build ([#14](https://github.com/nimble-123/origami/issues/14)) ([7aad17a](https://github.com/nimble-123/origami/commit/7aad17a965b54171005d99c20ba50a043cc18bc4))
* settle units left as written, and keep a mid-file U+FEFF off the first byte ([#22](https://github.com/nimble-123/origami/issues/22)) ([c6fee44](https://github.com/nimble-123/origami/commit/c6fee4449c7653b60d047153e372a9760a1dbde1))

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
