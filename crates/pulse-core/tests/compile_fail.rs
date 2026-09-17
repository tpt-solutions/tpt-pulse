//! Protocol sequencing is enforced by compilation, not runtime checks.

#[test]
fn invalid_protocol_operations() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/*.rs");
}
