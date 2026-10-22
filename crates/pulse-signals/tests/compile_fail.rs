//! Compile-fail checks for `#[signal_graph]` diagnostics.

#[test]
fn compile_fail() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/*.rs");
}
