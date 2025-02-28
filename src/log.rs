use crate::Error;
use gb_cpu_sim::{cpu, memory};
use owo_colors::{OwoColorize, Style};

use crate::test::{FailureReason, TestConfig};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SilenceLevel {
	#[default]
	None,
	Passing, // Silences passing messages when tests succeed.
	All,     // Silences all output unless an error occurs.
}

/// Tracks and prints test results.
pub struct Logger<'a> {
	silence_all: bool,
	silence_passing: bool,
	rom_path: &'a str,
	pub pass: u32,
	pub failure: u32,
}

pub struct TestLogger<'a, 'b> {
	logger: &'b mut Logger<'a>,
	name: &'b String,
	enable_breakpoints: bool,
}

impl<'a> Logger<'a> {
	#[must_use]
	pub fn new(silence_level: SilenceLevel, rom_path: &'a str) -> Logger<'a> {
		let (silence_all, silence_passing) = match silence_level {
			SilenceLevel::None => (false, false),
			SilenceLevel::Passing => (false, true),
			SilenceLevel::All => (true, true),
		};

		Logger {
			silence_all,
			silence_passing,
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
	#[must_use]
	pub fn finish(&self) -> bool {
		// When in SILENCE_ALL only print the final message if a test failed.
		if !self.silence_all || self.failure != 0 {
			let total = self.pass + self.failure;
			let style: Style = if total == 0 {
				Style::new().yellow()
			} else if self.pass == total {
				Style::new().green()
			} else if self.pass == 0 {
				Style::new().red()
			} else {
				Style::new().yellow()
			};

			println!(
				"{}{} {}/{} {}",
				self.rom_path.style(style),
				": All tests complete.".style(style),
				self.pass.style(style),
				(self.pass + self.failure).style(style),
				"passed.".style(style)
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
				"{}: {} {}",
				self.logger.rom_path.green(),
				self.name.green(),
				"passed".green()
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
			"{}: {} {}:\n{}\n{}",
			self.logger.rom_path.red(),
			self.name.red(),
			"failed".red(),
			match failure_reason {
				FailureReason::InvalidOpcode => "Invalid opcode".red(),
				FailureReason::Crash => "Crashed".red(),
				FailureReason::Timeout => "Timeout".red(),
			},
			cpu_state
		);
		self.logger.failure += 1;
	}
	pub fn incorrect(&mut self, msg: &Error) {
		print!(
			"{}: {} {}:\n{}",
			self.logger.rom_path.red(),
			"failed".red(),
			self.name.red(),
			msg,
		);
		self.logger.failure += 1;
	}
}
