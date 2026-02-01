# Contributing

## Branches
This project uses the following branch types:
  - `feature` is for adding, refactoring, or removing a feature.
  - `bugfix` for fixing a bug.
  - `hotfix` for temporary fixes or when bypassing normal testing in an emergency. Use `security` if it is a security related fix.
  - `security` for all security fixes.
  - `test` for experimenting outside of an issue/ticket.
  - `doc` for adding, changing or removing documentation.

Branche names should start with the branch type and include related issues in the path.
  - `feature/issue-142/add-http-support`
  - `test/refactor-core-io-read-file`

## Changelog

Add a changelog entry to CHANGELOG.md under the appropriate category.

Categories are based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Categories:
  - Security
  - Removed
  - Deprecated
  - Added
  - Changed
  - Fixed

Try to keep each changelog entry to a single line. Pull request actions check for changelog updates.

Changelog entries should placed under the correct category in `unreleased` and follow the format of:

`  - <short description> #<pr_num> (@<github_username>)`

Examples:

Under Added
  - Added http protocol support for port tests. #123 (@chadeldridge)

Under Changed
  - Refactored core::io::ReadFile to use idomatic methods. #547 (@chadeldridge)