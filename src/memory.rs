use gb_cpu_sim::memory;
use std::io::{Error, Write};

#[derive(Clone)]
pub struct AddressSpace<'a> {
	pub rom: &'a Vec<u8>,
	pub vram: [u8; 0x2000], // VRAM locking is not emulated as there is not PPU present.
	pub sram: [u8; 0x2000],
	pub wram: [u8; 0x2000],
	// Accessing echo ram will throw a warning.
	// OAM includes the 105 unused bytes of OAM; they will throw a warning.
	pub oam: [u8; 0x100],
	// All MMIO registers are special-cased; many serve no function.
	// HRAM does not include 0xFFFF (or IE register)
	pub hram: [u8; 0x7F],
}

impl memory::AddressSpace for AddressSpace<'_> {
	fn read(&self, address: u16) -> u8 {
		let address = address as usize;
		match address {
			0x0000..=0x3FFF => self.rom[address],
			0xC000..=0xDFFF => self.wram[address - 0xC000],
			0xFF80..=0xFFFE => self.hram[address - 0xFF80],
			_ => panic!("Unimplemented address range for 0x{address:04x}"),
		}
	}

	fn write(&mut self, address: u16, value: u8) {
		let address = address as usize;
		match address {
			0x0000..=0x3FFF => eprintln!("Wrote to ROM (MBC registers are not yet emulated)"),
			0xC000..=0xDFFF => self.wram[address - 0xC000] = value,
			0xFF80..=0xFFFE => self.hram[address - 0xFF80] = value,
			_ => panic!("Unimplemented address range for 0x{address:04x}"),
		}
	}
}

impl AddressSpace<'_> {
	#[must_use]
	pub fn with(rom: &Vec<u8>) -> AddressSpace {
		AddressSpace {
			rom,
			vram: [0; 0x2000],
			sram: [0; 0x2000],
			wram: [0; 0x2000],
			oam: [0; 0x100],
			hram: [0; 0x7F],
		}
	}

	/// Dumps the contents of memory to a buffer.
	///
	/// # Errors
	///
	/// Fails if the buffer could not be written to.
	pub fn dump<W: Write>(&self, mut file: W) -> Result<(), Error> {
		fn dump_memory<W: Write>(
			name: &str,
			start: usize,
			memory: &[u8],
			file: &mut W,
		) -> Result<(), Error> {
			// Print memory header
			writeln!(file, "[{name}]")?;

			// Chunk VRAM into 16 byte blocks, along with the address of each block
			let address_chunks = memory.chunks(16).zip((start..).step_by(16));

			// Print each chunk to file
			for (chunk, address) in address_chunks {
				let formatted_chunk = chunk
					.iter()
					.map(|x| format!("0x{x:02x}"))
					.collect::<Vec<String>>()
					.join(" ");

				writeln!(file, "0x{address:04x}: {formatted_chunk}")?;
			}

			// Add extra whitespace to separate the dumps a bit more
			writeln!(file)?;

			Ok(())
		}

		dump_memory("VRAM", 0x8000, &self.vram, &mut file)?;
		dump_memory("WRAM", 0xC000, &self.wram, &mut file)?;
		dump_memory("HRAM", 0xFF80, &self.hram, &mut file)?;

		Ok(())
	}
}
