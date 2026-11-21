# Statement catalog

Test data for `crates/origami_core/tests/catalog.rs`: every example statement
from the statement tests of [abaplint](https://github.com/abaplint/abaplint)
(`packages/core/test/abap/statements/*.ts`, one entry per distinct statement,
with its abaplint statement type) and every CDS source from its CDS parser
tests (`packages/core/test/cds/cds_parser.ts`).

* Source: abaplint commit `3a5e6dd79838896ad6970cfbc6afb5d84a8e8a50`
  (`@abaplint/core` 2.120.112, the version the lexer oracle uses). The
  catalog covers language versions up to `v816` (SAP_BASIS 8.16) and ABAP
  Cloud.
* Extracted statically (string literals read as text, concatenations
  joined, examples meant to fail excluded); nothing from abaplint was run.
* License: MIT, Copyright (c) 2015 Lars Hvam. See
  `THIRD_PARTY_NOTICES.md`.

The test checks every statement for internal errors, idempotency and false
structural errors, and caps the number of statements without a grammar
(W003). Statements of dynpro flow logic (`PROCESS …`, `FIELD …`, `CHAIN`)
are in the catalog but are not ABAP source; they stay W003.
