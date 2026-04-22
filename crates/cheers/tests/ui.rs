#[test]
fn signal_in_regular_attribute_is_rejected() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/signal_in_regular_attr.rs");
}

#[test]
fn big_int_in_js_context_is_rejected() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/big_int_in_js_context.rs");
}
