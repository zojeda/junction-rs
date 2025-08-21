use std::process::Command;

#[ignore]
#[test]
fn example_recommendation_system_runs_and_prints_counts() {
    // Run the example via cargo to ensure the non-test harness main executes.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let out = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--example",
            "recommendation_system",
            "-p",
            "junction-rs",
        ])
        .current_dir(manifest_dir)
        .output()
        .expect("failed to run recommendation_system example via cargo");

    assert!(
        out.status.success(),
        "example exited with non-zero status: {:?}\nstdout:\n{}\nstderr:\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Created 3 recommendations"),
        "missing created count in output: {}",
        stdout
    );
    assert!(
        stdout.contains("High-score recommendations: 2"),
        "missing high-score count in output: {}",
        stdout
    );
}
