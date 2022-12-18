use gb_cpu_sim::{cpu, memory};

use crate::log::TestLogger;
use crate::registers::Registers;

#[derive(Debug, Clone)]
pub struct TestConfig {
	pub name: String,

	pub crash_addresses: Vec<u16>,
	pub enable_breakpoints: bool,
	pub timeout: usize,

	pub initial: Registers,
	pub result: Option<Registers>,
}

pub enum TestResult {
	Pass,
	Incorrect(String),
	Failure(FailureReason),
}

#[derive(PartialEq, Eq)]
pub enum FailureReason {
	Crash,
	InvalidOpcode,
	Timeout,
}

impl TestConfig {
	pub fn run<A: memory::AddressSpace>(
		&self,
		cpu_state: &mut cpu::State<A>,
		logger: &mut TestLogger<'_, '_>,
	) -> TestResult {
		self.initial.configure(cpu_state);

		// Push the return address 0xFFFF onto the stack.
		// If pc == 0xFFFF the test is complete.
		// TODO: make the success address configurable.
		cpu_state.write(cpu_state.sp - 1, 0xFF);
		cpu_state.write(cpu_state.sp - 2, 0xFF);
		cpu_state.sp -= 2;

		let condition = loop {
			match cpu_state.tick() {
				cpu::TickResult::Ok => {}
				cpu::TickResult::Halt => break Ok(()),
				cpu::TickResult::Stop => break Ok(()),
				cpu::TickResult::Break => {
					logger.log_breakpoint(cpu_state);
				}
				cpu::TickResult::Debug => {
					logger.log_debug(cpu_state);
				}
				cpu::TickResult::InvalidOpcode => {
					break Err(FailureReason::InvalidOpcode);
				}
			}

			if cpu_state.pc == 0xFFFF {
				break Ok(());
			}

			if self.crash_addresses.contains(&cpu_state.pc) {
				break Err(FailureReason::Crash);
			}

			if cpu_state.cycles_elapsed >= self.timeout {
				break Err(FailureReason::Timeout);
			}
		};

		match condition {
			Err(failure_reason) => TestResult::Failure(failure_reason),
			Ok(()) => {
				if let Some(result) = &self.result {
					match result.compare(cpu_state) {
						Ok(()) => TestResult::Pass,
						Err(msg) => TestResult::Incorrect(msg),
					}
				} else {
					TestResult::Pass
				}
			}
		}
	}

	pub fn new(name: String) -> TestConfig {
		TestConfig {
			name,
			crash_addresses: vec![],
			enable_breakpoints: true,
			timeout: 65536,
			initial: Registers::new(),
			result: None,
		}
	}
	pub fn set_name(&mut self, name: String) {
		self.name = name;
	}
}
