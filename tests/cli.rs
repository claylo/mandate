use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir() -> PathBuf {
    let mut path = std::env::temp_dir();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    path.push(format!("mandate-test-{}-{}", std::process::id(), stamp));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

fn mandate_bin() -> PathBuf {
    if let Some(path) = option_env!("CARGO_BIN_EXE_mandate") {
        return PathBuf::from(path);
    }
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("debug");
    if cfg!(windows) {
        path.push("mandate.exe");
    } else {
        path.push("mandate");
    }
    path
}

#[test]
fn cli_file_input_writes_output() {
    let dir = temp_dir();
    let input = dir.join("input.md");
    let output = dir.join("out.1");

    fs::write(&input, "# mandate(1) -- Example\n\nParagraph.\n").expect("write input");

    let status = Command::new(mandate_bin())
        .args([
            "-i",
            input.to_str().unwrap(),
            "-p",
            "mandate",
            "-s",
            "1",
            "-t",
            "Test",
            "-o",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("run mandate");

    assert!(status.success());
    let roff = fs::read_to_string(output).expect("read output");
    assert!(roff.contains(".TH \"mandate\" \"1\" \"Test\""));
    assert!(roff.contains(".SH \"NAME\""));
}

#[test]
fn cli_stdin_input_writes_output() {
    let dir = temp_dir();
    let output = dir.join("out-stdin.1");

    let mut cmd = Command::new(mandate_bin());
    let mut child = cmd
        .args([
            "-i",
            "-",
            "-p",
            "mandate",
            "-s",
            "1",
            "-t",
            "Test",
            "-o",
            output.to_str().unwrap(),
        ])
        .stdin(Stdio::piped())
        .spawn()
        .expect("spawn mandate");

    {
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin
            .write_all(b"# mandate(1) -- Example\n\nParagraph.\n")
            .expect("write stdin");
    }

    let status = child.wait().expect("wait");
    assert!(status.success());
    let roff = fs::read_to_string(output).expect("read output");
    assert!(roff.contains(".TH \"mandate\" \"1\" \"Test\""));
}
