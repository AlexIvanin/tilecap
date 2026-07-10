use std::process::Command;

fn tilecap_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_tilecap"))
}

#[test]
fn help_exits_ok() {
    let output = tilecap_bin()
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("tilecap"));
}

#[test]
fn unknown_option_fails() {
    let output = tilecap_bin()
        .arg("--bogus")
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn mode_requires_argument() {
    let output = tilecap_bin()
        .arg("-m")
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn unknown_mode_fails() {
    let output = tilecap_bin()
        .args(["-m", "unknown"])
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn output_requires_argument() {
    let output = tilecap_bin()
        .arg("-o")
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn bad_geometry_fails() {
    let output = tilecap_bin()
        .args(["-r", "not-valid"])
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn full_screen_needs_display() {
    // On a headless CI, this will fail with "no display found"
    // On a desktop it should succeed or fail with X11 errors
    let output = tilecap_bin()
        .args(["-m", "full"])
        .output()
        .unwrap();

    if std::env::var("DISPLAY").is_ok() && std::env::var("WAYLAND_DISPLAY").is_err() {
        // On X11, it should either succeed or fail with an X11 error
        let stderr = String::from_utf8(output.stderr).unwrap();
        if !output.status.success() {
            assert!(
                stderr.contains("no display found")
                    || stderr.contains("failed to open display")
                    || stderr.contains("X11")
                    || stderr.contains("Connection"),
                "unexpected error: {stderr}"
            );
        }
    } else {
        // No display — should fail gracefully
        assert!(!output.status.success());
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains("no display found"));
    }
}
