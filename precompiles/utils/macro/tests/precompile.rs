

#[test]
fn ui() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/precompile/compile-fail/**/*.rs");
	t.pass("tests/precompile/pass/**/*.rs");
}

#[test]
fn expand() {
	macrotest::expand_without_refresh("tests/precompile/expand/**/*.rs");
}
