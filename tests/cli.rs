//! End-to-end tests over the built binary.
//!
//! The pasture itself needs a terminal, so what is checked here is everything
//! that works without one: the rendered frame, the CLI surface a keybinding or
//! a plugin action goes through, and the failure path when Herdr is not there.

use std::process::{Command, Output};

/// Run the binary with a Herdr that does not exist, so no test can reach the
/// live session by accident. Tests that need the bridge to fail rely on it too.
fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_herdr-sheep"))
        .args(args)
        .env("HERDR_BIN_PATH", "/nonexistent/herdr")
        .output()
        .expect("the binary runs")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("utf-8 stdout")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("utf-8 stderr")
}

#[test]
fn help_documents_every_way_in() {
    let output = run(&["--help"]);
    assert!(output.status.success(), "{}", stderr(&output));
    let help = stdout(&output);
    for expected in [
        "--open here",
        "--open side",
        "--snapshot",
        "--demo",
        "HERDR_SHEEP_FPS",
        "double click",
    ] {
        assert!(
            help.contains(expected),
            "--help never mentions {expected}:\n{help}"
        );
    }
}

#[test]
fn version_is_the_crate_version() {
    let output = run(&["--version"]);
    assert!(output.status.success());
    assert_eq!(
        stdout(&output).trim(),
        format!("herdr-sheep {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn a_demo_frame_fills_the_size_it_was_given_and_no_more() {
    let output = run(&["--demo", "100x38"]);
    assert!(output.status.success(), "{}", stderr(&output));
    let frame = stdout(&output);
    let rows: Vec<&str> = frame.lines().collect();

    assert_eq!(
        rows.len(),
        38,
        "a frame is exactly as tall as asked:\n{frame}"
    );
    for row in &rows {
        assert!(
            row.chars().count() <= 100,
            "row is wider than the pane: {row:?}"
        );
    }

    // Every zone, since the demo has a sheep in every state.
    for zone in ["GATE", "PEN", "PADDOCK", "MEADOW", "FOLD"] {
        assert!(frame.contains(zone), "no {zone} zone:\n{frame}");
    }
    // The mascot's prompt face, facing either way.
    assert!(
        frame.contains(">_") || frame.contains("_<"),
        "no sheep:\n{frame}"
    );
    // Fenced zones, and a barn standing in the fence along the horizon.
    assert!(frame.contains("|----"), "no fence:\n{frame}");
    assert!(frame.contains("|_|_|_|"), "no barn:\n{frame}");
}

#[test]
fn a_small_pane_falls_back_to_one_line_per_agent() {
    let output = run(&["--demo", "60x12"]);
    assert!(output.status.success(), "{}", stderr(&output));
    let frame = stdout(&output);
    assert!(
        !frame.contains("PADDOCK"),
        "the compact list has no zones:\n{frame}"
    );
    assert!(frame.contains("waiting on you"), "{frame}");
    assert!(frame.contains("mystery"), "every agent is listed:\n{frame}");
}

#[test]
fn bad_usage_exits_two_and_says_what_was_wrong() {
    for (args, expected) in [
        (vec!["--nope"], "--nope"),
        (vec!["--demo", "4x4"], "too small"),
        (vec!["--demo", "wide"], "WxH"),
        (vec!["--open"], "--open needs"),
        (vec!["--open", "sideways"], "here or side"),
        (vec!["--open", "side", "--demo"], "pick one"),
        (vec!["100x40"], "--snapshot or --demo"),
    ] {
        let output = run(&args);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{args:?} should be a usage error"
        );
        let message = stderr(&output);
        assert!(
            message.contains(expected),
            "{args:?} never explained {expected:?}:\n{message}"
        );
    }
}

#[test]
fn a_missing_herdr_binary_fails_loudly_instead_of_hanging() {
    // Both paths that call back into Herdr have to report a broken bridge
    // rather than drawing an empty pasture or exiting quietly.
    for args in [vec!["--snapshot", "80x24"], vec!["--open", "side"]] {
        let output = run(&args);
        assert_eq!(output.status.code(), Some(1), "{args:?} should fail");
        let message = stderr(&output);
        assert!(
            message.contains("cannot run") && message.contains("/nonexistent/herdr"),
            "{args:?} hid the broken bridge:\n{message}"
        );
    }
}
