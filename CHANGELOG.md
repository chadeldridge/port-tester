# Changelog

See [Contributing](CONTRIBUTING.md) for details on how to add to the Changelog.

## unreleased
### Added
  - Added "serde" feature for JSON output.
  - Added --json option to hold output until providing a final json report. #5 (@chadeldridge)
  - Added more thorough metrics module testing. #4 (@chadeldridge)
  - Added documentation for the Metrics module. #4 (@chadeldridge)
  - Added new Metrics struct to hold a list of MetricsResult, the MetricsSummary and Verbosity. #4 (@chadeldridge)
  - Added MetricsResult to hold attempt number, start, duration, and Status. #4 (@chadeldridge)
  - Added Status to record the Success or Failure of a connection attempt. #4 (@chadeldridge)
  - Added basic testing in all modules and the binary. #3 (@chadeldridge)
### Changed
  - Metrics became MetricsSummary. #4 (@chadeldridge)
  - All paths in Metrics that printed text now return String. #4 (@chadeldridge)
  - Renamed library from pt to port_tester. pt was already taken. #4 (@chadeldridge)
  - Separated binary and library functionality. #2 (@chadeldridge)
### Deprecated
### Removed
  - Removed `Arc<Mutex<>>` requirements for core::metrics::Metrics. #5 (@chadeldridge)
### Fixed
### Security

## v0.1.0 (2026-02-01)
### Added
  - Added base functionality of port open testing, metrics summary, and incremental summaries. #1 (@chadeldridge)
