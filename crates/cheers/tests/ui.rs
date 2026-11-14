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
fn async_fallback_extra_node_is_rejected() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/async_fallback_extra_node.rs");
}

#[test]
fn option_in_js_context_is_rejected() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/option_in_js_context.rs");
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
fn invalid_datastar_modifier_is_rejected() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/invalid_datastar_modifier.rs");
}

#[test]
fn execute_script_new_rejects_string() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/execute_script_new_rejects_string.rs");
}

#[test]
fn unsupported_signal_path_segment_is_rejected() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/unsupported_signal_path_segment.rs");
}

#[test]
fn show_without_initially_is_rejected() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/show_without_initially.rs");
}

#[test]
fn show_with_its_own_display_is_rejected() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/show_with_its_own_display.rs");
}

#[test]
fn known_element_with_wrong_kind_is_rejected() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/known_element_with_wrong_kind.rs");
}

#[test]
fn datastar_event_does_not_ambiguous_glob_import() {
    let tests = trybuild::TestCases::new();
    tests.pass("tests/ui/datastar_event_does_not_ambiguous_glob_import.rs");
}

#[test]
fn markup_in_script_is_rejected() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/markup_in_script.rs");
}

#[test]
fn markup_in_style_is_rejected() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/markup_in_style.rs");
}

#[test]
fn string_in_style_is_rejected() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/string_in_style.rs");
}

#[test]
fn html_value_in_script_is_rejected() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/html_value_in_script.rs");
}

#[test]
fn js_spread_in_datastar_object_is_rejected() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/js_spread_in_datastar_object.rs");
}

#[test]
fn spread_non_object_is_rejected() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/spread_non_object.rs");
}

#[test]
fn conditional_spread_is_rejected() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/conditional_spread.rs");
}

#[test]
fn empty_js_object_value_is_rejected() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/empty_js_object_value.rs");
}

#[test]
fn data_attribute_nested_group_error_is_reported() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/data_attribute_nested_group_error.rs");
}

#[test]
fn missing_data_name_is_reported() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/missing_data_name.rs");
}

#[test]
fn async_js_object_value_is_rejected() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/async_js_object_value.rs");
}
