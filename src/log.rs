use gb_cpu_sim::{cpu, memory};

use crate::test::{FailureReason, TestConfig};

pub const SILENCE_NONE: u8 = 0;
pub const SILENCE_PASSING: u8 = 1; // Silences passing messages when tests succeed.
pub const SILENCE_ALL: u8 = 2; // Silences all output unless an error occurs.

/// Tracks and prints test results.
pub struct Logger<'a> {
	silence_all: bool,
	silence_passing: bool,
	rom_path: &'a str,
	pass: u32,
	failure: u32,
}

pub struct TestLogger<'a, 'b> {
	logger: &'b mut Logger<'a>,
	name: &'b String,
	enable_breakpoints: bool,
}

impl<'a> Logger<'a> {
	pub fn new(silence_level: u8, rom_path: &str) -> Logger<'_> {
		Logger {
			silence_all: silence_level >= SILENCE_ALL,
			silence_passing: silence_level >= SILENCE_PASSING,
			rom_path,
			pass: 0,
			failure: 0,
		}
	}
	pub fn make_test<'b>(&'b mut self, config: &'b TestConfig) -> TestLogger<'a, 'b> {
		TestLogger {
			logger: self,
			name: &config.name,
			enable_breakpoints: config.enable_breakpoints,
		}
	}
	pub fn finish(self) -> bool {
		// When in SILENCE_ALL only print the final message if a test failed.
		if !self.silence_all || self.failure != 0 {
			println!(
				"{}: All tests complete. {}/{} passed.",
				self.rom_path,
				self.pass,
				self.pass + self.failure
			);
		}
		self.failure == 0
	}
}

impl<'a, 'b> TestLogger<'a, 'b> {
	pub fn log_breakpoint<A: memory::AddressSpace>(&mut self, cpu_state: &cpu::State<A>) {
		if self.enable_breakpoints {
			println!(
				"{}: BREAKPOINT in {} \n{cpu_state}",
				self.logger.rom_path, self.name
			);
		}
	}
	pub fn log_debug<A: memory::AddressSpace>(&mut self, cpu_state: &cpu::State<A>) {
		if self.enable_breakpoints {
			println!(
				"{}: DEBUG in {} \n{cpu_state}",
				self.logger.rom_path, self.name
			);
		}
	}
	pub fn pass(&mut self) {
		if !self.logger.silence_passing {
			println!(
				"\x1B[92m{}: {} passed\x1B[0m",
				self.logger.rom_path, self.name
			);
		}
		self.logger.pass += 1;
	}
	pub fn failure<A: memory::AddressSpace>(
		&mut self,
		failure_reason: &FailureReason,
		cpu_state: &cpu::State<A>,
	) {
		println!(
			"\x1B[91m{}: {} failed\x1B[0m:\n{}\n{}",
			self.logger.rom_path,
			self.name,
			match failure_reason {
				FailureReason::InvalidOpcode => "Invalid opcode",
				FailureReason::Crash => "Crashed",
				FailureReason::Timeout => "Timeout",
			},
			cpu_state
		);
		self.logger.failure += 1;
	}
	pub fn incorrect(&mut self, msg: &str) {
		print!(
			"\x1B[91m{}: {} failed\x1B[0m:\n{}",
			self.logger.rom_path, self.name, msg
		);
		self.logger.failure += 1;
	}
}
