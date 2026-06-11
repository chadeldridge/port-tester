# Release Process

Releases are driven by [`cargo-release`](https://github.com/crate-ci/cargo-release), which handles version bumping, CHANGELOG promotion, tagging, and pushing. The CI/CD pipeline then runs the full check suite, publishes build artifacts to GitHub Releases, and publishes the crate to crates.io automatically.

See [CONTRIBUTING.md](CONTRIBUTING.md) for branch and changelog conventions.

## Prerequisites

Install the required local tools:

```bash
cargo install cargo-release
cargo install cargo-audit
cargo install cargo-msrv
cargo install cargo-deny
cargo install cargo-hack
cargo install cargo-semver-checks
cargo install typos-cli
```

`rustfmt` and `clippy` are installed via rustup:

```bash
rustup component add rustfmt clippy
```

Add your crates.io API token to the environment (only needed if publishing manually):

```bash
export CARGO_REGISTRY_TOKEN=<your-token>
```

## Steps

### 1. Ensure main is green

All changes must be merged to `main` with CI passing before cutting a release.

### 2. Add CHANGELOG entries

All changes for this release must be recorded under `## [Unreleased]` in `CHANGELOG.md` following the format in [CONTRIBUTING.md](CONTRIBUTING.md). `cargo-release` will promote this section automatically.

### 3. Run cargo-release

From the project root, choose the version bump level:

```bash
cargo release patch   # bug fixes only
cargo release minor   # new features, backwards-compatible
cargo release major   # breaking changes
```

This runs the following sequence automatically, in order:

| Step | What happens |
|---|---|
| Pre-release hook | Runs `scripts/pre-release.sh` (full local check suite — see table below) |
| Version bump | Updates `version` in `Cargo.toml` and `Cargo.lock` |
| CHANGELOG promotion | Renames `## [Unreleased]` → `## [x.y.z] - YYYY-MM-DD`, inserts a fresh empty `## [Unreleased]` section above it |
| Release commit | `git commit` with all modified files |
| Tag | `git tag -a vx.y.z` |
| Push | Pushes commit and tag to `origin` |

The pre-release hook (`scripts/pre-release.sh`) runs these checks before any files are modified:

| Check | Command |
|---|---|
| Lockfile | `cargo update -p port_tester --locked` |
| Security audit | `cargo audit` |
| MSRV detection | `cargo msrv find --write-msrv` |
| Formatter | `cargo fmt --all --check` |
| Linter | `cargo clippy --workspace --all-targets --no-deps -- -D warnings` |
| Tests | `cargo test --locked --release` |
| License / dep audit | `cargo deny check bans licenses sources` |
| Spellcheck | `typos` |
| MSRV compatibility | `cargo hack check --all-targets --rust-version --workspace --ignore-private --locked` |
| SemVer compliance | `cargo semver-checks` |
| Smoke tests | `cargo run --locked -- 1` / `-v 1` / `-vv 1` |

If any check fails, `cargo-release` stops before making any commits or changes.

### 4. CI/CD pipeline

Pushing the tag matching `v*.*.*` triggers the full CI/CD pipeline:

1. All lint, test, and audit jobs run.
2. On success, the `build` job compiles release artifacts for all target platforms and attaches them to a GitHub Release.
3. The `publish` job then publishes the crate to crates.io using the `CARGO_REGISTRY_TOKEN` secret.

### 5. Verify

- Check the [Actions](https://github.com/chadeldridge/port-tester/actions) tab to confirm all jobs passed.
- Confirm the GitHub Release was created with the expected artifacts.
- Confirm the new version appears on [crates.io](https://crates.io/crates/port_tester).

---

## Notes

### CHANGELOG link conversion

`cargo-release` converts raw `#N (@user)` references to markdown links as part of the release commit, before the tag is pushed. Between releases, `CHANGELOG.md` on `main` retains the raw format — GitHub auto-links `#N` to the PR, so it remains readable.

### Dry run

To preview what `cargo-release` would do without making any changes:

```bash
cargo release minor --dry-run
```

### crates.io token

The `CARGO_REGISTRY_TOKEN` secret must be set in the repository's GitHub Actions secrets before the first release. Generate a token at <https://crates.io/settings/tokens> with the `publish-new` and `publish-update` scopes.
