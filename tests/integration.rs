#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

struct Fixture {
    _tempdir: TempDir,
    adb_path: PathBuf,
    state_file: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let tempdir = TempDir::new().expect("tempdir");
        let adb_path = tempdir.path().join("adb");
        let state_file = tempdir.path().join("state");
        let script = r#"#!/bin/sh
set -eu

SCENARIO="${ADB_QR_TEST_SCENARIO:-qr_success}"
STATE_FILE="${ADB_QR_STATE_FILE:?}"
SERVICE_NAME="${ADB_QR_TEST_SERVICE_NAME:-adb-qr-testsvc}"

command="${1:-}"
shift || true

case "$command" in
  version)
    echo "Android Debug Bridge version 1.0.41"
    ;;
  mdns)
    subcommand="${1:-}"
    if [ "$subcommand" = "check" ]; then
      echo "mdns daemon version [adb discovery 0.0.0]"
      exit 0
    fi

    if [ "$subcommand" = "services" ]; then
      echo "List of discovered mdns services"
      case "$SCENARIO" in
        qr_success|partial_success)
          printf '%s\t_adb-tls-pairing._tcp\t192.168.0.5:37123\n' "$SERVICE_NAME"
          ;;
        timeout)
          :
          ;;
      esac
      exit 0
    fi
    ;;
  pair)
    printf 'paired=1\n' > "$STATE_FILE"
    echo "Successfully paired to $1 [guid=adb-TESTGUID]"
    ;;
  devices)
    echo "List of devices attached"
    if [ -f "$STATE_FILE" ] && [ "$SCENARIO" != "partial_success" ]; then
      echo "adb-TESTGUID._adb-tls-connect._tcp	device"
    fi
    ;;
  *)
    echo "unsupported command: $command" >&2
    exit 1
    ;;
esac
"#;

        fs::write(&adb_path, script).expect("write fake adb");
        let mut permissions = fs::metadata(&adb_path).expect("metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&adb_path, permissions).expect("chmod");

        Self {
            _tempdir: tempdir,
            adb_path,
            state_file,
        }
    }

    fn run(&self, args: &[&str], scenario: &str) -> Output {
        let mut command = Command::new(binary_path());
        command.args(args);
        command.env("ADB_QR_STATE_FILE", &self.state_file);
        command.env("ADB_QR_TEST_SCENARIO", scenario);
        command.env("ADB_QR_TEST_SERVICE_NAME", "adb-qr-testsvc");
        command.env("ADB_QR_SERVICE_NAME_OVERRIDE", "adb-qr-testsvc");
        let should_pass_adb_path = !matches!(args.first(), Some(&"qr"));
        if should_pass_adb_path {
            command.arg("--adb-path");
            command.arg(&self.adb_path);
        }
        command.output().expect("run binary")
    }
}

fn binary_path() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_adb-qr"))
}

#[test]
fn qr_pair_happy_path_prints_success() {
    let fixture = Fixture::new();
    let output = fixture.run(&[], "qr_success");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "stdout: {stdout}");
    assert!(stdout.contains("Scan this QR code:"), "stdout: {stdout}");
    assert!(
        stdout.contains("Pairing succeeded and the device is visible in adb."),
        "stdout: {stdout}"
    );
}

#[test]
fn qr_pair_timeout_returns_timeout_exit_code() {
    let fixture = Fixture::new();
    let output = fixture.run(&["--timeout", "1"], "timeout");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(3), "stdout: {stdout}\nstderr: {stderr}");
    assert!(stderr.contains("timed out waiting for pairing service"), "stderr: {stderr}");
}

#[test]
fn partial_success_still_exits_zero() {
    let fixture = Fixture::new();
    let output = fixture.run(&[], "partial_success");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert_eq!(output.status.code(), Some(0), "stdout: {stdout}");
    assert!(
        stdout.contains("Pairing succeeded, but the device is not visible in adb yet."),
        "stdout: {stdout}"
    );
}

#[test]
fn qr_command_prints_payload() {
    let fixture = Fixture::new();
    let output = fixture.run(&["qr", "--print-payload"], "qr_success");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert_eq!(output.status.code(), Some(0), "stdout: {stdout}");
    assert!(stdout.contains("Service name: adb-qr-testsvc"), "stdout: {stdout}");
    assert!(stdout.contains("Payload: WIFI:T:ADB;S:adb-qr-testsvc;"), "stdout: {stdout}");
}

#[test]
fn missing_adb_returns_unsupported_exit_code() {
    let output = Command::new(binary_path())
        .args(["--adb-path", "/definitely/missing/adb"])
        .output()
        .expect("run binary");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(4), "stderr: {stderr}");
    assert!(stderr.contains("adb was not found"), "stderr: {stderr}");
}
