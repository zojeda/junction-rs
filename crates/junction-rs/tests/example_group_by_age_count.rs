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
fn example_group_by_age_count_runs_and_prints_ranges() {
    let exe = find_example_bin("group_by_age_count").expect(
        "example binary not found; run with `cargo test --all-targets -p junction-rs` to build examples",
    );
    let out = Command::new(exe)
        .output()
        .expect("failed to run group_by_age_count example");

    assert!(
        out.status.success(),
        "example exited with non-zero status: {:?}\nstdout:\n{}\nstderr:\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    // Expect lines for 30-40 and 40-50 buckets with counts 2 and 3 respectively (order not guaranteed)
    let has_30 = stdout.contains("range=30-40 count=2");
    let has_40 = stdout.contains("range=40-50 count=3");
    assert!(
        has_30 && has_40,
        "missing expected bucket counts in output: {}",
        stdout
    );
}
