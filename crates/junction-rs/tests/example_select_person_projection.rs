use std::path::Path;
use std::process::Command;

fn find_example_bin(name: &str) -> Option<String> {
    if let Ok(p) = std::env::var(format!("CARGO_BIN_EXE_{}", name)) {
        return Some(p);
    }
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let p1 = Path::new(manifest_dir)
        .join("../../target/debug/examples")
        .join(name);
    if p1.exists() {
        return Some(p1.to_string_lossy().into_owned());
    }
    let p2 = Path::new(manifest_dir)
        .join("../../target/debug")
        .join(name);
    if p2.exists() {
        return Some(p2.to_string_lossy().into_owned());
    }
    None
}

#[test]
fn example_select_person_projection_runs_and_prints_projected() {
    let exe = find_example_bin("select_person_projection").expect(
        "example binary not found; run with `cargo test --all-targets -p junction-rs` to build examples",
    );
    let out = Command::new(exe)
        .output()
        .expect("failed to run select_person_projection example");

    assert!(
        out.status.success(),
        "example exited with non-zero status: {:?}\nstdout:\n{}\nstderr:\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Got 2 projected rows"),
        "missing projected count in output: {}",
        stdout
    );
    assert!(
        stdout.contains("name: \"Tom\""),
        "missing 'Tom' in projection output: {}",
        stdout
    );
    assert!(
        stdout.contains("name: \"Jaime\""),
        "missing 'Jaime' in projection output: {}",
        stdout
    );
}
