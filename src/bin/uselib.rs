// Example of how evunit can be used as a rust library rather than a command line app.

use evunit::cpu::State as CpuState;
use evunit::log::Logger;
use evunit::log::SILENCE_NONE;
use evunit::memory::AddressSpace;
use evunit::open_rom;
use evunit::registers::Registers;
use evunit::test::TestConfig;
use std::process::exit;

fn main() {
	let mut test = TestConfig::new(String::from("Test 0"));
	// Initial state
	test.initial.a = Some(1);
	test.initial.b = Some(2);
	// Expected state
	let mut result = Registers::new();
	result.a = Some(3);
	test.result = Some(result);

	// Load the ROM
	let rom_path = "test/test.gb";
	let rom = open_rom(rom_path);
	let address_space = AddressSpace::with(&rom);

	// Prepare test
	let mut cpu = CpuState::new(address_space);
	let mut logger = Logger::new(SILENCE_NONE, rom_path);
	let mut test_logger = logger.make_test(&test);

	// Run and exit
	test.run(&mut cpu, &mut test_logger);

	if !logger.finish() {
		exit(1);
	}
}
