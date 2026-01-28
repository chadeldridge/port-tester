# Contributing

## Branches
This project uses the following branch types:
  - `feature` is for adding, refactoring, or removing a feature.
  - `bugfix` for fixing a bug.
  - `hotfix` for temporary fixes or when bypassing normal testing in an emergency. Use `security` if it is a security related fix.
  - `security` for all security fixes.
  - `test` for experimenting outside of an issue/ticket.
  - `doc` for adding, changing or removing documentation.

Branche names should be start with the branch type and include related issues in the path.
  - `feature/issue-142/add-http-support`
  - `test/refactor-core-io-read-file`

## Changelog

Add a changelog entry in `changelog.d/` named `<identifier>.<category>.md`
  - With Issue: `issue-<number>.<category>.md`
  - Without Issue: `<branch-description>.<category>.md`

Where \<category\> is one of:
  - `security`
  - `removed`
  - `deprecated`
  - `added`
  - `changed`
  - `fixed`

Documentation and typo updates can skip changelog checks by adding the label `no-changelog` in the Pull Request.

Try to keep each changelog entry to a single line. If multiple changelog lines of the same type are needed, use the format:

`<branch-description>-<number>.<type>.md`

The GitHub workflow will automatically added the PR # and contributor's username upon release.

### Examples:

  - Branch: `feature/issue-142/add-http-support`
    - File: `issue-142.added.md`
    - Content: `Added http protocol support for port tests`
  - Branch: `test/refactor-core-io-read-file`
    - File: `test-refactor-core-io-read-file.changed.md`
    - Content: `Refactored core::io::ReadFile to use idomatic methods`
