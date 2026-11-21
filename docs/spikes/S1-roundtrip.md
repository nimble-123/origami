# Spike S1: Round trip through SAP

**Status:** open. Needs an SAP system with write access (see Open Question 5 in
the design).

**Decision it feeds:** if any SAP path rewrites layout, the CI `--check` gate
reports false failures, and origami is positioned as an authoring tool rather
than a CI gate.

## Procedure

1. Pick three objects from the corpus: one class with local classes, one CDS
   view entity, one behavior definition.
2. Change their layout by hand (re-indent, move line breaks). Keep tokens
   identical.
3. **Path A, ADT for VS Code:** open each object, paste the changed source,
   save, activate. Read the source back (close and reopen the editor). Diff it
   against what you pasted.
4. **Path B, abapGit:** commit the changed files to a branch, pull the branch
   into the system with abapGit, activate, then serialize back (stage and
   commit from abapGit). Diff the serialized files against the commit.
5. Repeat step 3 with ADT's own format command, to learn whether it needs a
   system connection (input for S2).

## Record

| Item | Value |
|---|---|
| System, release, client | |
| ADT for VS Code version | |
| abapGit version | |
| Path A: layout preserved? (per object) | |
| Path B: layout preserved? (per object) | |
| Notes | |

## Decision

To be written after the experiment, then copied into Open Question 1 of
`docs/designs/origami.md`.
