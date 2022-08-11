use crate::memory::AddressSpace;

#[derive(Debug)]
pub struct Flags {
	pub value: u8
}

impl Flags {
	pub fn get_z(&self) -> bool { self.value & 0b10000000 != 0 }
	pub fn get_n(&self) -> bool { self.value & 0b01000000 != 0 }
	pub fn get_h(&self) -> bool { self.value & 0b00100000 != 0 }
	pub fn get_c(&self) -> bool { self.value & 0b00010000 != 0 }
	pub fn set_z(&mut self, value: bool) {
		self.value = self.value & 0b01110000 | (value as u8) << 7;
	}
	pub fn set_n(&mut self, value: bool) {
		self.value = self.value & 0b10110000 | (value as u8) << 6;
	}
	pub fn set_h(&mut self, value: bool) {
		self.value = self.value & 0b11010000 | (value as u8) << 5;
	}
	pub fn set_c(&mut self, value: bool) {
		self.value = self.value & 0b11100000 | (value as u8) << 4;
	}
}

#[derive(Debug)]
pub struct State {
	// Primary CPU Registers
	pub a: u8, pub f: Flags,
	pub b: u8, pub c: u8,
	pub d: u8, pub e: u8,
	pub h: u8, pub l: u8,
	pub pc: u16,
	pub sp: u16,

	// Total number of M-Cycles that have passed during this CPU's life.
	pub cycles_processed: usize,
}

impl State {
	pub fn get_af(&self) -> u16 { (self.f.value as u16) | (self.a as u16) << 8 }
	pub fn get_bc(&self) -> u16 { (self.c as u16) | (self.b as u16) << 8 }
	pub fn get_de(&self) -> u16 { (self.e as u16) | (self.d as u16) << 8 }
	pub fn get_hl(&self) -> u16 { (self.l as u16) | (self.h as u16) << 8 }
	pub fn set_af(&mut self, value: u16) {
		self.f.value = (value & 0xFF) as u8;
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

	fn read_pc(&mut self, address_space: &AddressSpace) -> u8 {
		let value = address_space.read(self.pc);
		self.pc = u16::wrapping_add(self.pc, 1);
		self.cycles_processed += 1;
		value
	}

	// Returns true upon test completion.
	pub fn tick(&mut self, address_space: &mut AddressSpace) -> bool {
		let opcode = self.read_pc(&address_space);

		fn inc_r8(register: &mut u8, flags: &mut Flags) {
			let old_register = *register;
			*register = u8::wrapping_add(*register, 1);
			flags.set_z(*register == 0);
			flags.set_n(false);
			flags.set_h(old_register & 0xF == 0xF);
		}

		fn dec_r8(register: &mut u8, flags: &mut Flags) {
			let old_register = *register;
			*register = u8::wrapping_sub(*register, 1);
			flags.set_z(*register == 0);
			flags.set_n(true);
			flags.set_h(old_register & 0xF == 0xF);
		}

		fn rlc_r8(register: &mut u8, flags: &mut Flags) {
			let old_register = *register;
			*register = *register << 1 | flags.get_c() as u8;
			flags.set_z(*register == 0);
			flags.set_n(false);
			flags.set_h(false);
			flags.set_c(old_register & 0b10000000 != 0);
		}

		fn rl_r8(register: &mut u8, flags: &mut Flags) {
			let old_register = *register;
			*register = u8::rotate_left(*register, 1);
			flags.set_z(*register == 0);
			flags.set_n(false);
			flags.set_h(false);
			flags.set_c(old_register & 0b10000000 != 0);
		}

		fn rrc_r8(register: &mut u8, flags: &mut Flags) {
			let old_register = *register;
			*register = *register >> 1 | (flags.get_c() as u8) << 7;
			flags.set_z(*register == 0);
			flags.set_n(false);
			flags.set_h(false);
			flags.set_c(old_register & 1 != 0);
		}

		fn rr_r8(register: &mut u8, flags: &mut Flags) {
			let old_register = *register;
			*register = u8::rotate_right(*register, 1);
			flags.set_z(*register == 0);
			flags.set_n(false);
			flags.set_h(false);
			flags.set_c(old_register & 1 != 0);
		}

		fn add_hl_r16(value: u16, cpu: &mut State) {
			let old_hl = cpu.get_hl();
			cpu.set_hl(u16::wrapping_add(cpu.get_hl(), value));
			cpu.f.set_n(false);
			cpu.f.set_h((old_hl & 0xFFF + value > 0xFFF) == true);
			cpu.cycles_processed += 1;
		}

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
				inc_r8(&mut self.b, &mut self.f);
			},
			/* dec b */ 0x05 => {
				dec_r8(&mut self.b, &mut self.f);
			},
			/* ld b, u8 */ 0x06 => {
				self.b = self.read_pc(&address_space);
			},
			/* rlca */ 0x07 => {
				rlc_r8(&mut self.a, &mut self.f);
				self.f.set_z(false);
			},
			/* ld [u16], sp */ 0x08 => {
				let pointer = (self.read_pc(&address_space) as u16) | (self.read_pc(&address_space) as u16) << 8;
				address_space.write(pointer, (self.sp & 0xFF) as u8);
				address_space.write(pointer + 1, (self.sp >> 8) as u8);
				self.cycles_processed += 4;
			},
			/* add hl, bc */ 0x09 => {
				add_hl_r16(self.get_bc(), self);
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
				inc_r8(&mut self.c, &mut self.f);
			},
			/* dec c */ 0x0D => {
				dec_r8(&mut self.c, &mut self.f);
			},
			/* ld c, u8 */ 0x0E => {
				self.c = self.read_pc(&address_space);
			},
			/* rrca */ 0x0F => {
				rrc_r8(&mut self.a, &mut self.f);
				self.f.set_z(false);
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
				inc_r8(&mut self.d, &mut self.f);
			},
			/* dec d */ 0x15 => {
				dec_r8(&mut self.d, &mut self.f);
			},
			/* ld d, u8 */ 0x16 => {
				self.d = self.read_pc(&address_space);
			},
			/* rla */ 0x17 => {
				rl_r8(&mut self.a, &mut self.f);
				self.f.set_z(false);
			},
			/* jr u8 */ 0x18 => {
				self.pc = i16::wrapping_add(self.pc as i16, self.read_pc(&address_space) as i16) as u16;
				self.cycles_processed += 1;
			},
			/* add hl, de */ 0x19 => {
				add_hl_r16(self.get_de(), self);
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
				inc_r8(&mut self.e, &mut self.f);
			},
			/* dec e */ 0x1D => {
				dec_r8(&mut self.e, &mut self.f);
			},
			/* ld e, u8 */ 0x1E => {
				self.e = self.read_pc(&address_space);
			},
			/* rra */ 0x1F => {
				rr_r8(&mut self.a, &mut self.f);
				self.f.set_z(false);
			},
			/* jr nz */ 0x20 => {
				let offset = self.read_pc(&address_space) as i16;
				if !self.f.get_z() {
					self.pc = i16::wrapping_add(self.pc as i16, offset) as u16;
					self.cycles_processed += 1;
				}
			},
			/* ld hl, u16 */ 0x21 => {
				self.e = self.read_pc(&address_space);
				self.d = self.read_pc(&address_space);
			},
			/* ld [hli], a */ 0x22 => {
				address_space.write(self.get_hl(), self.a);
				self.set_hl(u16::wrapping_add(self.get_hl(), 1));
				self.cycles_processed += 1;
			},
			/* inc hl */ 0x23 => {
				self.set_hl(u16::wrapping_add(self.get_hl(), 1));
				self.cycles_processed += 1;
			},
			/* inc h */ 0x24 => {
				inc_r8(&mut self.h, &mut self.f);
			},
			/* dec h */ 0x25 => {
				dec_r8(&mut self.h, &mut self.f);
			},
			/* ld h, u8 */ 0x26 => {
				self.h = self.read_pc(&address_space);
			},
			/* daa */ 0x27 => {
				println!("Sorry, daa is unimplemented");
			},
			/* jr z */ 0x28 => {
				let offset = self.read_pc(&address_space) as i16;
				if self.f.get_z() {
					self.pc = i16::wrapping_add(self.pc as i16, offset) as u16;
					self.cycles_processed += 1;
				}
			},
			/* add hl, hl */ 0x29 => {
				add_hl_r16(self.get_hl(), self);
			},
			/* ld a, [hli] */ 0x2A => {
				self.a = address_space.read(self.get_hl());
				self.set_hl(u16::wrapping_add(self.get_hl(), 1));
				self.cycles_processed += 1;
			},
			/* dec hl */ 0x2B => {
				self.set_hl(u16::wrapping_sub(self.get_hl(), 1));
				self.cycles_processed += 1;
			},
			/* inc l */ 0x2C => {
				inc_r8(&mut self.l, &mut self.f);
			},
			/* dec l */ 0x2D => {
				dec_r8(&mut self.l, &mut self.f);
			},
			/* ld l, u8 */ 0x2E => {
				self.l = self.read_pc(&address_space);
			},
			/* cpl */ 0x2F => {
				self.a = !self.a;
			},
			/* jr nc */ 0x30 => {
				let offset = self.read_pc(&address_space) as i16;
				if !self.f.get_c() {
					self.pc = i16::wrapping_add(self.pc as i16, offset) as u16;
					self.cycles_processed += 1;
				}
			},
			/* ld sp, u16 */ 0x31 => {
				self.sp = self.read_pc(&address_space) as u16 | (self.read_pc(&address_space) << 8) as u16;
			},
			/* ld [hld], a */ 0x32 => {
				address_space.write(self.get_hl(), self.a);
				self.set_hl(u16::wrapping_sub(self.get_hl(), 1));
				self.cycles_processed += 1;
			},
			/* inc sp */ 0x33 => {
				self.sp = u16::wrapping_add(self.sp, 1);
				self.cycles_processed += 1;
			},
			/* inc [hl] */ 0x34 => {
				let value = address_space.read(self.get_hl());
				inc_r8(&mut value, &mut self.f);
				address_space.write(self.get_hl(), value);
				self.cycles_processed += 2;
			},
			/* dec [hl] */ 0x35 => {
				let value = address_space.read(self.get_hl());
				dec_r8(&mut value, &mut self.f);
				address_space.write(self.get_hl(), value);
				self.cycles_processed += 2;
			},
			/* ld [hl], u8 */ 0x36 => {
				address_space.write(self.get_hl(), self.read_pc());
				self.cycles_processed += 1;
			},
			/* scf */ 0x37 => {
				self.f.set_n(false);
				self.f.set_h(false);
				self.f.set_c(true);
			},
			/* jr c */ 0x38 => {
				let offset = self.read_pc(&address_space) as i16;
				if self.f.get_c() {
					self.pc = i16::wrapping_add(self.pc as i16, offset) as u16;
					self.cycles_processed += 1;
				}
			},
			/* add hl, sp */ 0x39 => {
				add_hl_r16(self.sp, self);
			},
			/* ld a, [hld] */ 0x3A => {
				self.a = address_space.read(self.get_hl());
				self.set_hl(u16::wrapping_sub(self.get_hl(), 1));
				self.cycles_processed += 1;
			},
			/* dec sp */ 0x3B => {
				self.sp = u16::wrapping_sub(self.sp, 1);
				self.cycles_processed += 1;
			},
			/* inc a */ 0x3C => {
				inc_r8(&mut self.a, &mut self.f);
			},
			/* dec a */ 0x3D => {
				dec_r8(&mut self.a, &mut self.f);
			},
			/* ld a, u8 */ 0x3E => {
				self.a = self.read_pc(&address_space);
			},
			/* ccf */ 0x3F => {
				self.f.set_n(false);
				self.f.set_h(false);
				self.f.set_c(!self.f.get_c());
			},
			_ => panic!("Invalid opcode"),
		}

		return false;
	}

	pub fn new() -> State {
		State {
			a: 0,
			f: Flags { value: 0 },
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
