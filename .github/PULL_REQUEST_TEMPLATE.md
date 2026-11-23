<!--
The title becomes the commit on main. It must be a Conventional Commit:
type(scope): what changes, in the imperative. See CONTRIBUTING.md.
A change to formatted output is `style(...)` and names its STYLE.md rule ID.
-->

## Summary

What changes and why. Link the issue (`Fixes #123`).

## Evidence

How you know it works: tests, `cargo xtask ci` output, before and after.

- **Before:**
- **After:**

## Merge Danger

**Door:** one-way or two-way. Can the change be reverted after it ships?

**Blast Radius:** what else changes, and for whom (narrow, docs, formatted output, CLI contract, ...).
