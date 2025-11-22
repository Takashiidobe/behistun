use datatest_stable as datatest;
use std::{
    fs, io,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::Once,
};

const QEMU: &str = "qemu-m68k-static";

static INIT: Once = Once::new();

fn ensure_csmith_bins() {
    INIT.call_once(|| {
        let status = Command::new("make")
            .arg("test-csmith-bins")
            .status()
            .expect("Failed to run 'make test-csmith-bins'");
        assert!(status.success(), "make test-csmith-bins failed");
    });
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RunLog {
    stdout: String,
    stderr: String,
    status: i32,
}

fn tool_available(bin: &str) -> bool {
    Command::new(bin)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn run_qemu(exe: &Path, args: &[String]) -> io::Result<Output> {
    let mut cmd = Command::new(QEMU);
    cmd.arg(exe).args(args);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).output()
}

fn run_interp(exe: &Path, args: &[String]) -> io::Result<Output> {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_behistun"));
    cmd.arg(exe).args(args);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).output()
}

fn to_runlog(out: Output) -> RunLog {
    RunLog {
        stdout: String::from_utf8_lossy(&out.stdout).trim().to_string(),
        stderr: String::from_utf8_lossy(&out.stderr).to_string(),
        status: out.status.code().unwrap_or(-1),
    }
}

fn source_to_binary(src_path: &Path) -> PathBuf {
    // Convert test-csmith/foo.c to test-bins/csmith/foo
    let stem = src_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("out");
    PathBuf::from("test-bins/csmith").join(stem)
}

fn run_case(path: &Path) -> datatest::Result<()> {
    ensure_csmith_bins();

    if !tool_available(QEMU) {
        eprintln!("skipping {} (missing {})", path.display(), QEMU);
        return Ok(());
    }

    let exe = source_to_binary(path);

    // Check if binary exists
    if !exe.exists() {
        panic!(
            "Binary {} not found. Make sure 'make test-csmith-bins' succeeded.",
            exe.display()
        );
    }

    let args = load_args(path);

    let run_out_ref = run_qemu(&exe, &args)?;
    let reference = to_runlog(run_out_ref);

    let run_out_interp = run_interp(&exe, &args)?;
    let mine = to_runlog(run_out_interp);

    if mine != reference {
        let mut msg = String::new();
        use std::fmt::Write;
        writeln!(&mut msg, "\n=== MISMATCH for {} ===", path.display()).ok();

        if mine.status != reference.status {
            writeln!(
                &mut msg,
                "Exit code differs: interp={} qemu={}",
                mine.status, reference.status
            )
            .ok();
        }
        if mine.stdout != reference.stdout {
            writeln!(&mut msg, "\n--- stdout (interp) ---\n{}", mine.stdout).ok();
            writeln!(&mut msg, "\n--- stdout (qemu)  ---\n{}", reference.stdout).ok();
        }
        if mine.stderr != reference.stderr {
            writeln!(&mut msg, "\n--- stderr (interp) ---\n{}", mine.stderr).ok();
            writeln!(&mut msg, "\n--- stderr (qemu)  ---\n{}", reference.stderr).ok();
        }

        panic!("{msg}");
    }

    Ok(())
}

datatest::harness! {
    { test = run_case, root = "./test-csmith", pattern = r#"^.*\.c$"# },
}

fn load_args(path: &Path) -> Vec<String> {
    let args_path = path.with_extension("args");
    if let Ok(text) = fs::read_to_string(args_path) {
        text.split_whitespace().map(|s| s.to_string()).collect()
    } else {
        Vec::new()
    }
}
