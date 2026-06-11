# Changelog

See [Contributing](CONTRIBUTING.md) for details on how to add to the Changelog.

## [Unreleased]
### Security
### Fixed
  - Fixed broken intra-doc links in `Metrics` documentation. #7 (@chadeldridge)
  - Fixed terminology inconsistencies in documentation and help text. #7 (@chadeldridge)
  - Standardized `chrono` duration types and formatting in metrics tests. #7 (@chadeldridge)
### Added
  - Added cargo release process and documentation. #11 (@chadeldridge)
  - Added Issues and Pull Request templates. #10 (@chadeldridge)
  - Added poke executable. Poke always performs a single TCP connection attempt. #8 (@chadeldridge)
  - Added dependabot to automate dependency updates. #8 (@chadeldridge)
  - Added support for testing multiple resolved IP addresses in sequence. #7 (@chadeldridge)
  - Added `scripts/qa.sh` for end-to-end functional testing. #7 (@chadeldridge)
  - Added documentation linting. #7 (@chadeldridge)
  - Added documentation for the Host module to align with Rust API Guidelines. #6 (@chadeldridge)
  - Added thorough testing for the Host module. #6 (@chadeldridge)
  - Added "serde" feature for JSON output. #5 (@chadeldridge)
  - Added --json option to hold output until providing a final json report. #5 (@chadeldridge)
  - Added more thorough metrics module testing. #4 (@chadeldridge)
  - Added documentation for the Metrics module. #4 (@chadeldridge)
  - Added new Metrics struct to hold a list of MetricsResult, the MetricsSummary and Verbosity. #4 (@chadeldridge)
  - Added MetricsResult to hold attempt number, start, duration, and Status. #4 (@chadeldridge)
  - Added Status to record the Success or Failure of a connection attempt. #4 (@chadeldridge)
  - Added basic testing in all modules and the binary. #3 (@chadeldridge)
### Changed
  - Updated Contributing guidelines. #10 (@chadeldridge)
  - Updated dependency crates: typos #9 (dependabot)
  - Updated dependencies crates. chrono, clap, ctrlc, env_logger, log, serde_json #8 (@chadeldridge)
  - `scripts/pre-release.sh` now runs `scripts/qa.sh` as a required check. #8 (@chadeldridge)
  - Refactored CICD GitHub Actions workflow to include poke. #8 (@chadeldridge)
  - Refactored `Metrics::result` to use checked_sub instead of a match. Returns `None` if `seq` < 1. #7 (@chadeldridge)
  - Refactored `Host` to store multiple addresses and updated connection logic to iterate through them. #7 (@chadeldridge)
  - Updated `README.md` to align with current CLI arguments and JSON output format. #7 (@chadeldridge)
  - Updated Metrics module documentation to align with Rust API Guidelines. #6 (@chadeldridge)
  - Made `Host` fields private and added getter methods to follow Rust API Guidelines. #6 (@chadeldridge)
  - Metrics became MetricsSummary. #4 (@chadeldridge)
  - All paths in Metrics that printed text now return String. #4 (@chadeldridge)
  - Renamed library from pt to port_tester. pt was already taken. #4 (@chadeldridge)
  - Separated binary and library functionality. #2 (@chadeldridge)
### Deprecated
### Removed
  - Removed unused is_err field in `MetricsResultJSON`. #6 (@chadeldridge)
  - Removed `Arc<Mutex<>>` requirements for core::metrics::Metrics. #5 (@chadeldridge)

## [v0.1.0] - 2026-02-01
### Security
### Fixed
### Added
  - Added base functionality of port open testing, metrics summary, and incremental summaries. #1 (@chadeldridge)
### Changed
### Deprecated
### Removed

[Unreleased]: https://github.com/chadeldridge/port-tester/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/chadeldridge/port-tester/compare/f30cf8c...v0.1.0
