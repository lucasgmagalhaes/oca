use super::*;

// `extract_first_json_object` is a private helper, so unlike the rest of this module's tests
// (see `core/tests/loudness.rs` for those — they only exercise the public API), this
// one has to stay a unit test: an integration test in `tests/` can't see non-`pub` items.
#[test]
fn extract_first_json_object_ignores_unbalanced_braces_before_it() {
    let text = "log } line\n{\"a\":1}\nmore text";
    assert_eq!(extract_first_json_object(text), Some("{\"a\":1}"));
}
