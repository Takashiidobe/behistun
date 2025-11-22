use datatest_stable as datatest;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Once,
};

static INIT: Once = Once::new();

fn ensure_integration_bins() {
    INIT.call_once(|| {
        let status = Command::new("make")
            .arg("test-integration-bins")
            .status()
            .expect("Failed to run 'make test-integration-bins'");
        assert!(status.success(), "make test-integration-bins failed");
    });
}

fn run_interp(exe: &Path, args: &[String]) -> std::io::Result<std::process::Output> {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_behistun"));
    cmd.arg(exe).args(args);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).output()
}

fn source_to_binary(src_path: &Path) -> PathBuf {
    // Strip extension and replace test-integration with test-bins/integration
    let without_ext = src_path.with_extension("");
    let bin_path_str = without_ext
        .to_str()
        .unwrap()
        .replace("test-integration", "test-bins/integration");

    PathBuf::from(bin_path_str)
}

fn run_case(path: &Path) -> datatest::Result<()> {
    ensure_integration_bins();

    let exe = source_to_binary(path);

    // Check if binary exists
    if !exe.exists() {
        panic!(
            "Binary {} not found. Make sure 'make test-integration-bins' succeeded.",
            exe.display()
        );
    }

    let args = load_args(path);

    let output = run_interp(&exe, &args)?;

    // Just check that the test returned 0 (success)
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "Test {} failed with exit code {}\nstdout: {}\nstderr: {}",
            path.display(),
            output.status.code().unwrap_or(-1),
            stdout,
            stderr
        );
    }

    Ok(())
}

datatest::harness! {
    { test = run_case, root = "./test-integration", pattern = r#"^.*\.(c|S)$"# },
}

fn load_args(path: &Path) -> Vec<String> {
    let args_path = path.with_extension("args");
    if let Ok(text) = fs::read_to_string(args_path) {
        text.split_whitespace().map(|s| s.to_string()).collect()
    } else {
        Vec::new()
    }
}
