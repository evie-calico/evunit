// Example of how evunit can be used as a rust library rather than a command line app.
// TOOD: move this to examples/
use evunit::prelude::*;
use std::process::exit;

fn main() {
	let mut tests = Vec::new();

	for a in 0..=128 {
		for b in 0..=127 {
			let mut test = TestConfig::new(format!("Test {a} + {b}"));

			// Initial state
			test.initial = Registers::new().with_a(a).with_b(b);

			// Expected state
			test.result = Some(Registers::new().with_a(a + b));

			tests.push(test);
		}
	}

	let result = run_tests("test/test.gb", &tests, SilenceLevel::Passing);

	if result.is_err() {
		exit(1);
	}
}
