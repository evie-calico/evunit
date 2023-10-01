#![doc = include_str!("../README.md")]
#![warn(clippy::all, clippy::pedantic)]

pub use gb_cpu_sim::cpu;

pub mod log;
pub mod memory;
pub mod prelude;
pub mod registers;
pub mod test;

use crate::log::{Logger, SilenceLevel};
use crate::memory::AddressSpace;
use crate::test::TestConfig;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::process::exit;

/// Library Error type.
#[derive(Debug, thiserror::Error)]
pub enum Error {
	/// Returned when a unit test failed according to the user's conditions.
	#[error("{0} tests failed")]
	TestsFailed(u32),
	#[error("{0}")]
	CompareFailed(registers::CompareResult),
}

type Result<T> = std::result::Result<T, Error>;

#[must_use]
pub fn open_rom(path: &str) -> Vec<u8> {
	let mut rom = Vec::<u8>::new();
	File::open(path)
		.unwrap_or_else(|msg| {
			eprintln!("Failed to open {path}: {msg}");
			exit(1)
		})
		.read_to_end(&mut rom)
		.unwrap_or_else(|error| {
			eprintln!("Failed to read {path}: {error}");
			exit(1);
		});
	if rom.len() < 0x4000 {
		rom.resize(0x4000, 0xFF);
	}
	rom
}

#[must_use]
#[deprecated]
pub fn read_symfile(path: &Option<String>) -> HashMap<String, (u32, u16)> {
	open_symfile(path.as_deref().map(AsRef::<Path>::as_ref))
}

#[must_use]
pub fn open_symfile(path: Option<&Path>) -> HashMap<String, (u32, u16)> {
	let mut symfile = HashMap::new();

	if let Some(path) = path.as_ref() {
		let dpath = path.display();
		let file = File::open(path).unwrap_or_else(|error| {
			eprintln!("Failed to open {dpath}: {error}");
			exit(1);
		});
		let symbols = BufReader::new(file)
			.lines()
			.map(|line| {
				line.unwrap_or_else(|error| {
					eprintln!("Error reading {dpath}: {error}");
					exit(1);
				})
			})
			.enumerate()
			.filter_map(|(n, line)| {
				gb_sym_file::parse_line(&line).map(|parse_result| {
					parse_result.unwrap_or_else(|parse_error| {
						eprintln!("Failed to parse {dpath} line {}: {parse_error}", n + 1);
						exit(1);
					})
				})
			})
			// We are only interested in banked symbols
			.filter_map(|(name, loc)| match loc {
				gb_sym_file::Location::Banked(bank, addr) => Some((name, (bank, addr))),
				_ => None,
			});
		symfile.extend(symbols);
	}
	symfile
}

/// Run all provided unit tests using a given ROM.
///
/// # Errors
///
/// Returns an error if any of the tests failed.
pub fn run_tests(rom_path: &str, tests: &[TestConfig], silence_level: SilenceLevel) -> Result<()> {
	// Load the ROM
	let rom = open_rom(rom_path);
	let address_space = AddressSpace::with(&rom);
	let mut logger = Logger::new(silence_level, rom_path);

	for test in tests {
		// Prepare test
		let mut cpu = cpu::State::new(address_space.clone());
		let mut test_logger = logger.make_test(test);

		// Run and exit
		test.run(&mut cpu, &mut test_logger);
	}

	if logger.finish() {
		Ok(())
	} else {
		Err(Error::TestsFailed(logger.failure))
	}
}
