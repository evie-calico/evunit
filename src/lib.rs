pub use gb_cpu_sim::cpu;
pub mod log;
pub mod memory;
pub mod registers;
pub mod test;

use std::fs::File;
use std::io::Read;
use std::process::exit;

pub fn open_rom(path: &str) -> Vec<u8> {
	let mut rom = Vec::<u8>::new();
	File::open(path).unwrap_or_else(|msg| {
		eprintln!("Failed to open {path}: {msg}");
		exit(1)
	}).read_to_end(&mut rom).unwrap_or_else(|error| {
		eprintln!("Failed to read {path}: {error}");
		exit(1);
	});
	if rom.len() < 0x4000 {
		rom.resize(0x4000, 0xFF);
	}
	rom
}
