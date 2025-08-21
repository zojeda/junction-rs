#[test]
fn edge_validation_errors_are_reported() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/edge_missing_in.rs");
    t.compile_fail("tests/ui/edge_wrong_type.rs");
}
