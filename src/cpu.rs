use crate::memory::AddressSpace;

#[derive(Debug)]
pub struct State {
	// Primary CPU Registers
	pub a: u8, pub f: u8,
	pub b: u8, pub c: u8,
	pub d: u8, pub e: u8,
	pub h: u8, pub l: u8,
	pub pc: u16,
	pub sp: u16,

	// Total number of M-Cycles that have passed during this CPU's life.
	pub cycles_processed: usize,
}

impl State {
	pub fn get_af(&self) -> u16 { (self.f as u16) | (self.a as u16) << 8 }
	pub fn get_bc(&self) -> u16 { (self.c as u16) | (self.b as u16) << 8 }
	pub fn get_de(&self) -> u16 { (self.e as u16) | (self.d as u16) << 8 }
	pub fn get_hl(&self) -> u16 { (self.l as u16) | (self.h as u16) << 8 }
	pub fn get_zf(&self) -> bool { self.f & 0b10000000 != 0 }
	pub fn get_nf(&self) -> bool { self.f & 0b01000000 != 0 }
	pub fn get_hf(&self) -> bool { self.f & 0b00100000 != 0 }
	pub fn get_cf(&self) -> bool { self.f & 0b00010000 != 0 }
	pub fn set_af(&mut self, value: u16) {
		self.f = (value & 0xFF) as u8;
		self.a = (value >> 8) as u8;
	}
	pub fn set_bc(&mut self, value: u16) {
		self.c = (value & 0xFF) as u8;
		self.b = (value >> 8) as u8;
	}
	pub fn set_de(&mut self, value: u16) {
		self.e = (value & 0xFF) as u8;
		self.d = (value >> 8) as u8;
	}
	pub fn set_hl(&mut self, value: u16) {
		self.l = (value & 0xFF) as u8;
		self.h = (value >> 8) as u8;
	}
	pub fn set_zf(&mut self, value: bool) {
		let value = value as u8;
		self.f = self.f & 0b01110000 | value << 7;
	}
	pub fn set_nf(&mut self, value: bool) {
		let value = value as u8;
		self.f = self.f & 0b10110000 | value << 6;
	}
	pub fn set_hf(&mut self, value: bool) {
		let value = value as u8;
		self.f = self.f & 0b11010000 | value << 5;
	}
	pub fn set_cf(&mut self, value: bool) {
		let value = value as u8;
		self.f = self.f & 0b11100000 | value << 4;
	}

	fn read_pc(&mut self, address_space: &AddressSpace) -> u8 {
		let value = address_space.read(self.pc);
		self.pc = u16::wrapping_add(self.pc, 1);
		self.cycles_processed += 1;
		value
	}

	// Returns true upon test completion.
	pub fn tick(&mut self, address_space: &mut AddressSpace) -> bool {
		let opcode = self.read_pc(&address_space);

		match opcode {
			/* nop */ 0x00 => {},
			/* ld bc, u16 */ 0x01 => {
				self.c = self.read_pc(&address_space);
				self.b = self.read_pc(&address_space);
			},
			/* ld [bc], a */ 0x02 => {
				address_space.write(self.get_bc(), self.a);
				self.cycles_processed += 1;
			},
			/* inc bc */ 0x03 => {
				self.set_bc(u16::wrapping_add(self.get_bc(), 1));
				self.cycles_processed += 1;
			},
			/* inc b */ 0x04 => {
				let old_b = self.b;
				self.b = u8::wrapping_add(self.b, 1);
				self.set_zf(self.b != 0);
				self.set_nf(false);
				self.set_hf(old_b & 0xF == 0xF);
			},
			/* dec b */ 0x05 => {
				let old_b = self.b;
				self.b = u8::wrapping_sub(self.b, 1);
				self.set_zf(self.b != 0);
				self.set_nf(true);
				self.set_hf(old_b & 0x1F == 0x10);
			},
			/* ld b, u8 */ 0x06 => {
				self.b = self.read_pc(&address_space);
			},
			/* rlca */ 0x07 => {
				let old_a = self.a;
				self.a = self.a << 1 | self.get_cf() as u8;
				self.set_zf(false);
				self.set_nf(false);
				self.set_hf(false);
				self.set_cf(old_a & 0b10000000 != 0);
			},
			/* ld [u16], sp */ 0x08 => {
				let pointer = (self.read_pc(&address_space) as u16) | (self.read_pc(&address_space) as u16) << 8;
				address_space.write(pointer, (self.sp & 0xFF) as u8);
				address_space.write(pointer + 1, (self.sp >> 8) as u8);
				self.cycles_processed += 4;
			},
			/* add hl, bc */ 0x09 => {
				let old_hl = self.get_hl();
				self.set_hl(u16::wrapping_add(self.get_hl(), self.get_bc()));
				self.set_nf(false);
				self.set_hf((old_hl & 0xFFF + self.get_bc() > 0xFFF) == true);
				self.cycles_processed += 1;
			},
			/* ld a, [bc] */ 0x0A => {
				self.a = address_space.read(self.get_bc());
				self.cycles_processed += 1;
			},
			/* dec bc */ 0x0B => {
				self.set_bc(u16::wrapping_sub(self.get_bc(), 1));
				self.cycles_processed += 1;
			},
			/* inc c */ 0x0C => {
				let old_c = self.c;
				self.c = u8::wrapping_add(self.c, 1);
				self.set_zf(self.c != 0);
				self.set_nf(false);
				self.set_hf(old_c & 0xF == 0xF);
			},
			/* dec c */ 0x0D => {
				let old_c = self.c;
				self.c = u8::wrapping_sub(self.c, 1);
				self.set_zf(self.c != 0);
				self.set_nf(true);
				self.set_hf(old_c & 0x1F == 0x10);
			},
			/* ld c, u8 */ 0x0E => {
				self.c = self.read_pc(&address_space);
			},
			/* rrca */ 0x0F => {
				let old_a = self.a;
				self.a = self.a >> 1 | (self.get_cf() as u8) << 7;
				self.set_zf(false);
				self.set_nf(false);
				self.set_hf(false);
				self.set_cf(old_a & 0b00000001 != 0);
			},
			/* stop */ 0x10 => {
				self.read_pc(&address_space);
				return true;
			},
			/* ld de, u16 */ 0x11 => {
				self.e = self.read_pc(&address_space);
				self.d = self.read_pc(&address_space);
			},
			/* ld [de], a */ 0x12 => {
				address_space.write(self.get_de(), self.a);
				self.cycles_processed += 1;
			},
			/* inc de */ 0x13 => {
				self.set_de(u16::wrapping_add(self.get_de(), 1));
				self.cycles_processed += 1;
			},
			/* inc d */ 0x14 => {
				let old_d = self.d;
				self.d = u8::wrapping_add(self.d, 1);
				self.set_zf(self.d != 0);
				self.set_nf(false);
				self.set_hf(old_d & 0xF == 0xF);
			},
			/* dec d */ 0x15 => {
				let old_d = self.d;
				self.d = u8::wrapping_sub(self.d, 1);
				self.set_zf(self.d != 0);
				self.set_nf(true);
				self.set_hf(old_d & 0x1F == 0x10);
			},
			/* ld d, u8 */ 0x16 => {
				self.d = self.read_pc(&address_space);
			},
			/* rla */ 0x17 => {
				self.set_zf(false);
				self.set_nf(false);
				self.set_hf(false);
				self.set_cf(self.a & 0b10000000 != 0);
				self.a = u8::rotate_left(self.a);
			},
			/* jr u8 */ 0x18 => {
				self.pc = u16::wrapping_add(self.pc, self.read_pc(&address_space) as i16);
				self.cycles_processed += 1;
			},
			/* add hl, de */ 0x19 => {
				let old_hl = self.get_hl();
				self.set_hl(u16::wrapping_add(self.get_hl(), self.get_de()));
				self.set_nf(false);
				self.set_hf((old_hl & 0xFFF + self.get_de() > 0xFFF) == true);
				self.cycles_processed += 1;
			},
			/* ld a, [de] */ 0x1A => {
				self.a = address_space.read(self.get_de());
				self.cycles_processed += 1;
			},
			/* dec de */ 0x1B => {
				self.set_de(u16::wrapping_sub(self.get_de(), 1));
				self.cycles_processed += 1;
			},
			/* inc e */ 0x1C => {
				let old_e = self.c;
				self.e = u8::wrapping_add(self.e, 1);
				self.set_zf(self.e != 0);
				self.set_nf(false);
				self.set_hf(old_e & 0xF == 0xF);
			},
			/* dec e */ 0x1D => {
				let old_e = self.e;
				self.e = u8::wrapping_sub(self.e, 1);
				self.set_zf(self.e != 0);
				self.set_nf(true);
				self.set_hf(self.e & 0x1F == 0x10);
			},
			/* ld e, u8 */ 0x1E => {
				self.e = self.read_pc(&address_space);
			},
			/* rra */ 0x1F => {
				self.set_zf(false);
				self.set_nf(false);
				self.set_hf(false);
				self.set_cf(self.a & 0b00000001 != 0);
				self.a = u8::rotate_right(self.a);
			},
			/* jr nz */ 0x20 => {
				let offset = self.read_pc(&address_space) as i16;
				if !self.get_zf() {
					self.pc = u16::wrapping_add(self.pc, offset);
					send.cycles_processed += 1;
				}
			},
			/* ld hl, u16 */ 0x21 => {
				self.e = self.read_pc(&address_space);
				self.d = self.read_pc(&address_space);
			},
			/* ld [hli], a */ 0x22 => {
				address_space.write(self.get_hl(), self.a);
				self.hl = u16::wrapping_add(self.hl, 1);
				self.cycles_processed += 1;
			},
			/* inc hl */ 0x23 => {
				self.set_hl(u16::wrapping_add(self.get_hl(), 1));
				self.cycles_processed += 1;
			},
			/* inc h */ 0x24 => {
				let old_h = self.h;
				self.h = u8::wrapping_add(self.h, 1);
				self.set_zf(self.h != 0);
				self.set_nf(false);
				self.set_hf(old_h & 0xF == 0xF);
			},
			/* dec h */ 0x25 => {
				let old_h = self.h;
				self.h = u8::wrapping_sub(self.h, 1);
				self.set_zf(self.h != 0);
				self.set_nf(true);
				self.set_hf(old_h & 0x1F == 0x10);
			},
			/* ld h, u8 */ 0x26 => {
				self.h = self.read_pc(&address_space);
			},
			/* daa */ 0x27 => {
				println!("Sorry, daa is unimplemented");
			},
			/* jr z */ 0x28 => {
				let offset = self.read_pc(&address_space) as i16;
				if self.get_zf() {
					self.pc = u16::wrapping_add(self.pc, offset);
					send.cycles_processed += 1;
				}
			},
			/* add hl, hl */ 0x19 => {
				let old_hl = self.get_hl();
				self.set_hl(u16::wrapping_add(self.get_hl(), self.get_hl()));
				self.set_nf(false);
				self.set_hf((old_hl & 0xFFF + self.get_hl() > 0xFFF) == true);
				self.cycles_processed += 1;
			},
			/* ld a, [hli] */ 0x1A => {
				self.a = address_space.read(self.get_hl());
				self.hl = u16::wrapping_add(self.hl, 1);
				self.cycles_processed += 1;
			},
			/* dec hl */ 0x1B => {
				self.set_hl(u16::wrapping_sub(self.get_hl(), 1));
				self.cycles_processed += 1;
			},
			/* inc e */ 0x1C => {
				let old_e = self.c;
				self.e = u8::wrapping_add(self.e, 1);
				self.set_zf(self.e != 0);
				self.set_nf(false);
				self.set_hf(old_e & 0xF == 0xF);
			},
			/* dec e */ 0x1D => {
				let old_e = self.e;
				self.e = u8::wrapping_sub(self.e, 1);
				self.set_zf(self.e != 0);
				self.set_nf(true);
				self.set_hf(self.e & 0x1F == 0x10);
			},
			/* ld e, u8 */ 0x1E => {
				self.e = self.read_pc(&address_space);
			},
			/* cpl */ 0x1F => {
				self.a = ~self.a;
			},
			_ => panic!("Invalid opcode"),
		}

		return false
	}

	pub fn new() -> State {
		State {
			a: 0,
			f: 0,
			b: 0,
			c: 0,
			d: 0,
			e: 0,
			h: 0,
			l: 0,
			pc: 0,
			sp: 0,
			cycles_processed: 0,
		}
	}
}
