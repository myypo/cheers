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

#[test]
fn unsupported_ref_expr_is_rejected() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/unsupported_ref_expr.rs");
}

#[test]
fn unregistered_datastar_event_is_rejected() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/unregistered_datastar_event.rs");
}

#[test]
fn datastar_event_does_not_ambiguous_glob_import() {
    let tests = trybuild::TestCases::new();
    tests.pass("tests/ui/datastar_event_does_not_ambiguous_glob_import.rs");
}
