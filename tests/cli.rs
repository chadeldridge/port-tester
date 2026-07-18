//! End-to-end CLI tests for the `pt` and `poke` binaries.
//!
//! These run the real binaries as subprocesses (via `assert_cmd`) and assert on exit
//! codes and output. They cover behavior that cannot be exercised by in-process unit
//! tests, notably the argument/validation paths where the binaries call
//! `std::process::exit`.
//!
//! Tests that need live network access (DNS, outbound TCP/TLS) are marked so they only run
//! with `--features network-tests`; the rest are hermetic and safe to run offline.

use assert_cmd::Command;
use predicates::prelude::*;

/// A command for the named binary in this crate.
fn bin(name: &str) -> Command {
    Command::cargo_bin(name).expect("binary should be built by cargo")
}

// ---------------------------------------------------------------------------
// Help / version (regression snapshots)
// ---------------------------------------------------------------------------

#[test]
fn pt_help_matches_snapshot() {
    bin("pt")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::diff(include_str!("fixtures/pt-help.txt")));
}

#[test]
fn poke_help_matches_snapshot() {
    bin("poke")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::diff(include_str!("fixtures/poke-help.txt")));
}

#[test]
fn pt_version_reports_crate_version() {
    bin("pt")
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

// ---------------------------------------------------------------------------
// Argument validation (hermetic: these exit before any network access)
// ---------------------------------------------------------------------------

#[test]
fn pt_missing_host_is_usage_error() {
    // clap emits its own usage error with exit code 2.
    bin("pt")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("Usage:"));
}

#[test]
fn pt_invalid_host_is_options_error() {
    bin("pt")
        .arg("bad_host!!")
        .assert()
        .code(3)
        .stderr(predicate::str::contains("Invalid host format"));
}

#[test]
fn pt_invalid_port_is_options_error() {
    bin("pt")
        .arg("google.com:0")
        .assert()
        .code(3)
        .stderr(predicate::str::contains("Invalid port"));
}

#[test]
fn pt_conflicting_verbosity_is_usage_error() {
    // The verbosity flags share a clap arg group, so clap rejects the combination at
    // parse time (exit 2) before the binary's own check runs.
    bin("pt")
        .args(["-q", "-s", "google.com"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn pt_http_only_flag_without_http_mode_is_error() {
    bin("pt")
        .args(["--http-success", "google.com"])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("require --http"));
}

#[test]
fn poke_insecure_without_http_mode_is_error() {
    bin("poke")
        .args(["-k", "google.com"])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("require --http"));
}

// ---------------------------------------------------------------------------
// Connection behavior — failure paths (hermetic: loopback refuses instantly)
// ---------------------------------------------------------------------------

#[test]
fn pt_closed_port_exits_failure() {
    // 127.0.0.1:1 is essentially always closed; the connection is refused locally.
    bin("pt")
        .args(["127.0.0.1", "1", "-c", "1", "-t", "1", "-s"])
        .assert()
        .code(1);
}

#[test]
fn poke_closed_port_exits_failure() {
    bin("poke")
        .args(["127.0.0.1", "1", "-t", "1", "-s"])
        .assert()
        .code(1);
}

// ---------------------------------------------------------------------------
// Connection behavior — success/JSON/HTTP paths (require live network)
// ---------------------------------------------------------------------------

#[test]
#[cfg_attr(not(feature = "network-tests"), ignore = "requires network")]
fn pt_open_port_exits_success() {
    bin("pt")
        .args(["8.8.8.8", "53", "-c", "1", "-s"])
        .assert()
        .success();
}

#[test]
#[cfg_attr(not(feature = "network-tests"), ignore = "requires network")]
fn pt_json_output_has_expected_shape() {
    bin("pt")
        .args(["8.8.8.8", "53", "-c", "1", "--json"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("\"addrs\"")
                .and(predicate::str::contains("\"metrics\""))
                .and(predicate::str::starts_with("{")),
        );
}

#[test]
#[cfg_attr(not(feature = "network-tests"), ignore = "requires network")]
fn pt_http_get_exits_success() {
    bin("pt")
        .args(["--http", "-c", "1", "-s", "google.com"])
        .assert()
        .success();
}

#[test]
#[cfg_attr(not(feature = "network-tests"), ignore = "requires network")]
fn poke_open_port_exits_success() {
    bin("poke").args(["8.8.8.8", "53", "-s"]).assert().success();
}

#[test]
#[cfg_attr(not(feature = "network-tests"), ignore = "requires network")]
fn poke_https_rejects_bad_cert() {
    // Expired certificate must fail verification without --insecure.
    bin("poke")
        .args(["--https", "-s", "expired.badssl.com"])
        .assert()
        .code(1);
}

#[test]
#[cfg_attr(not(feature = "network-tests"), ignore = "requires network")]
fn poke_https_insecure_accepts_bad_cert() {
    // The same host succeeds once verification is disabled.
    bin("poke")
        .args(["--https", "-k", "-s", "expired.badssl.com"])
        .assert()
        .success();
}
