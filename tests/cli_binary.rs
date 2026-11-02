//! Spawns the built `route-info-builder` binary to cover the CLI entry point.

use std::process::Command;

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_route-info-builder"))
}

#[test]
fn binary_reports_version_and_help() {
    let version = binary().arg("--version").output().expect("binary must run");
    assert!(version.status.success());
    let stdout = String::from_utf8(version.stdout).expect("utf8");
    assert!(stdout.contains("route-info-builder"));
    assert!(stdout.contains("0.2.0"));

    let help = binary().arg("--help").output().expect("binary must run");
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("Usage:"));
}

#[test]
fn binary_generates_from_a_fixture_directory() {
    let dir = tempfile::tempdir().expect("tempdir");
    let controllers = dir.path().join("controllers");
    std::fs::create_dir_all(&controllers).expect("controllers dir");
    std::fs::write(
        controllers.join("users.rs"),
        "fn routes() -> Router {\n  Router::new().route(\"/api/users\", get(list_users))\n}\n",
    )
    .expect("fixture");

    let output = binary()
        .args([
            "generate",
            "--controllers",
            controllers.to_str().expect("utf8"),
            "--rust-output",
            dir.path().join("links.rs").to_str().expect("utf8"),
        ])
        .output()
        .expect("binary must run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(dir.path().join("links.rs").exists());
}

#[test]
fn binary_fails_on_missing_controllers() {
    let output = binary()
        .args(["generate", "--controllers", "/definitely/not/here"])
        .output()
        .expect("binary must run");
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("error:"));
}
