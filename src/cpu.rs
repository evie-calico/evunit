use crate::memory::AddressSpace;
use std::fmt;

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

impl fmt::Display for Flags {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}{}{}{}",
			if self.get_z() {'z'} else {'-'},
			if self.get_n() {'n'} else {'-'},
			if self.get_h() {'h'} else {'-'},
			if self.get_c() {'c'} else {'-'},
		)
	}
}

pub enum TickResult {
	Ok,
	Break,
	Debug,
	Halt,
	Stop,
	InvalidOpcode,
}

pub struct State {
	// Primary CPU Registers
	pub a: u8, pub f: Flags,
	pub b: u8, pub c: u8,
	pub d: u8, pub e: u8,
	pub h: u8, pub l: u8,
	pub pc: u16,
	pub sp: u16,

	pub ei: bool,

	// Total number of M-Cycles that have passed during this CPU's life.
	pub cycles_elapsed: usize,

	address_space: AddressSpace,
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

	// Passthroughs for address_space.read/write()
	pub fn read(&self, address: u16) -> u8 { self.address_space.read(address) }
	pub fn write(&mut self, address: u16, value: u8) { self.address_space.write(address, value); }

	fn read_pc(&mut self) -> u8 {
		let value = self.address_space.read(self.pc);
		self.pc = u16::wrapping_add(self.pc, 1);
		self.cycles_elapsed += 1;
		value
	}

	// Returns true upon test completion.
	pub fn tick(&mut self) -> TickResult {
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
			*register = register.rotate_left(1) | flags.get_c() as u8;
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
			*register = register.rotate_right(1) | (flags.get_c() as u8) << 7;
			flags.set_z(*register == 0);
			flags.set_n(false);
			flags.set_h(false);
			flags.set_c(old_register & 1 != 0);
		}

		fn sla_r8(register: u8, flags: &mut Flags) -> u8  {
			flags.set_c(register & 0x80 != 0);
			flags.set_n(false);
			flags.set_h(false);
			let result = register << 1;
			flags.set_z(result == 0);
			result
		}

		fn sra_r8(register: u8, flags: &mut Flags) -> u8  {
			flags.set_c(register & 1 != 0);
			flags.set_n(false);
			flags.set_h(false);
			let result = register >> 1 | register & 0x80;
			flags.set_z(result == 0);
			result
		}

		fn swap_r8(register: u8, flags: &mut Flags) -> u8 {
			let result = register.rotate_right(4);
			flags.set_z(result == 0);
			flags.set_n(false);
			flags.set_h(false);
			flags.set_c(false);
			result
		}

		fn srl_r8(register: u8, flags: &mut Flags) -> u8  {
			flags.set_c(register & 1 != 0);
			flags.set_n(false);
			flags.set_h(false);
			let result = register >> 1;
			flags.set_z(result == 0);
			result
		}

		fn bit_r8(register: u8, bit: u8, flags: &mut Flags) {
			flags.set_z(register & 1 << bit == 0);
			flags.set_n(false);
			flags.set_h(true);
		}

		fn set_r8(register: u8, bit: u8) -> u8 { register | 1 << bit }
		fn res_r8(register: u8, bit: u8) -> u8 { register & !(1 << bit) }

		fn add_hl_r16(value: u16, cpu: &mut State) {
			let old_hl = cpu.get_hl();
			cpu.set_hl(u16::wrapping_add(cpu.get_hl(), value));
			cpu.f.set_n(false);
			cpu.f.set_h((old_hl & 0xFFF + value & 0xFFF > 0xFFF) == true);
			cpu.f.set_c(old_hl > cpu.get_hl());
			cpu.cycles_elapsed += 1;
		}

		fn add_a_r8(value: u8, cpu: &mut State) {
			let old_a = cpu.a;
			cpu.a = u8::wrapping_add(cpu.a, value);
			cpu.f.set_z(cpu.a == 0);
			cpu.f.set_n(false);
			cpu.f.set_h(((old_a & 0xF) + (value & 0xF) > 0xF) == true);
			cpu.f.set_c(old_a > cpu.a);
		}

		fn adc_a_r8(value: u8, cpu: &mut State) {
			let carry = cpu.f.get_c() as u8;
			let old_a = cpu.a;
			cpu.a = u8::wrapping_add(cpu.a, value);
			cpu.a = u8::wrapping_add(cpu.a, carry);
			cpu.f.set_z(cpu.a == 0);
			cpu.f.set_n(false);
			cpu.f.set_h((old_a & 0xF + value & 0xF + carry > 0xF) == true);
			cpu.f.set_c(old_a as u16 + value as u16 + carry as u16 > 0xFF);
		}

		fn sub_a_r8(value: u8, cpu: &mut State) {
			let old_a = cpu.a;
			cpu.a = u8::wrapping_sub(cpu.a, value);
			cpu.f.set_z(cpu.a == 0);
			cpu.f.set_n(true);
			cpu.f.set_h((((old_a & 0xF) as i8) < ((value & 0xF) as i8)) == true);
			cpu.f.set_c(old_a < cpu.a);
		}

		fn sbc_a_r8(value: u8, cpu: &mut State) {
			let carry = cpu.f.get_c() as u8;
			let old_a = cpu.a;
			cpu.a = u8::wrapping_add(cpu.a, value);
			cpu.a = u8::wrapping_add(cpu.a, carry);
			cpu.f.set_z(cpu.a == 0);
			cpu.f.set_n(true);
			cpu.f.set_h((((old_a & 0xF) as i8) < (((value & 0xF) + carry) as i8)) == true);
			cpu.f.set_c((old_a as i16 - value as i16 - carry as i16) < 0);
		}

		fn and_a_r8(value: u8, cpu: &mut State) {
			cpu.a &= value;
			cpu.f.set_z(cpu.a == 0);
			cpu.f.set_n(false);
			cpu.f.set_h(true);
			cpu.f.set_c(false);
		}

		fn xor_a_r8(value: u8, cpu: &mut State) {
			cpu.a ^= value;
			cpu.f.set_z(cpu.a == 0);
			cpu.f.set_n(false);
			cpu.f.set_h(false);
			cpu.f.set_c(false);
		}

		fn or_a_r8(value: u8, cpu: &mut State) {
			cpu.a |= value;
			cpu.f.set_z(cpu.a == 0);
			cpu.f.set_n(false);
			cpu.f.set_h(false);
			cpu.f.set_c(false);
		}

		fn cp_a_r8(value: u8, cpu: &mut State) {
			let old_a = cpu.a;
			let result = u8::wrapping_sub(cpu.a, value);
			cpu.f.set_z(result == 0);
			cpu.f.set_n(true);
			cpu.f.set_h((((old_a & 0xF) as i8) < ((value & 0xF) as i8)) == true);
			cpu.f.set_c(value > result);
		}

		fn push(value: u16, cpu: &mut State) {
			cpu.sp = u16::wrapping_sub(cpu.sp, 1);
			cpu.write(cpu.sp, (value & 0xFF) as u8);
			cpu.sp = u16::wrapping_sub(cpu.sp, 1);
			cpu.write(cpu.sp, (value >> 8) as u8);
			cpu.cycles_elapsed += 3;
		}

		fn pop(cpu: &mut State) -> u16 {
			let mut result = (cpu.read(cpu.sp) as u16) << 8;
			cpu.sp += 1;
			result |= cpu.read(cpu.sp) as u16;
			cpu.sp += 1;
			cpu.cycles_elapsed += 2;
			result
		}

		fn call_cc(condition: bool, cpu: &mut State,) {
			if condition {
				push(cpu.pc + 2, cpu);
				cpu.pc = (cpu.read_pc() as u16) | (cpu.read_pc() as u16) << 8;
				cpu.cycles_elapsed += 1;
			} else {
				cpu.read_pc();
				cpu.read_pc();
			}
		}

		fn ret_cc(condition: bool, cpu: &mut State) {
			if condition {
				cpu.pc = pop(cpu);
				cpu.cycles_elapsed += 2; // pop already takes care of 2 extra cycles.
			} else {
				cpu.cycles_elapsed += 1;
			}
		}

		fn jp_cc(condition: bool, cpu: &mut State) {
			if condition {
				cpu.pc = (cpu.read_pc() as u16) | (cpu.read_pc() as u16) << 8;
				cpu.cycles_elapsed += 1;
			} else {
				cpu.read_pc();
				cpu.read_pc();
			}
		}

		fn jr_cc(condition: bool, cpu: &mut State) {
			let offset = cpu.read_pc() as i8;
			if condition {
				cpu.pc = i16::wrapping_add(cpu.pc as i16, offset as i16) as u16;
				cpu.cycles_elapsed += 1;
			}
		}

		match self.read_pc() {
			/* nop */ 0x00 => {},
			/* ld bc, u16 */ 0x01 => {
				self.c = self.read_pc();
				self.b = self.read_pc();
			},
			/* ld [bc], a */ 0x02 => {
				self.write(self.get_bc(), self.a);
				self.cycles_elapsed += 1;
			},
			/* inc bc */ 0x03 => {
				self.set_bc(u16::wrapping_add(self.get_bc(), 1));
				self.cycles_elapsed += 1;
			},
			/* inc b */ 0x04 => {
				inc_r8(&mut self.b, &mut self.f);
			},
			/* dec b */ 0x05 => {
				dec_r8(&mut self.b, &mut self.f);
			},
			/* ld b, u8 */ 0x06 => {
				self.b = self.read_pc();
			},
			/* rlca */ 0x07 => {
				rlc_r8(&mut self.a, &mut self.f);
				self.f.set_z(false);
			},
			/* ld [u16], sp */ 0x08 => {
				let pointer = (self.read_pc() as u16) | (self.read_pc() as u16) << 8;
				self.write(pointer, (self.sp & 0xFF) as u8);
				self.write(pointer + 1, (self.sp >> 8) as u8);
				self.cycles_elapsed += 4;
			},
			/* add hl, bc */ 0x09 => {
				add_hl_r16(self.get_bc(), self);
			},
			/* ld a, [bc] */ 0x0A => {
				self.a = self.read(self.get_bc());
				self.cycles_elapsed += 1;
			},
			/* dec bc */ 0x0B => {
				self.set_bc(u16::wrapping_sub(self.get_bc(), 1));
				self.cycles_elapsed += 1;
			},
			/* inc c */ 0x0C => {
				inc_r8(&mut self.c, &mut self.f);
			},
			/* dec c */ 0x0D => {
				dec_r8(&mut self.c, &mut self.f);
			},
			/* ld c, u8 */ 0x0E => {
				self.c = self.read_pc();
			},
			/* rrca */ 0x0F => {
				rrc_r8(&mut self.a, &mut self.f);
				self.f.set_z(false);
			},
			/* stop */ 0x10 => {
				self.read_pc();
				return TickResult::Stop;
			},
			/* ld de, u16 */ 0x11 => {
				self.e = self.read_pc();
				self.d = self.read_pc();
			},
			/* ld [de], a */ 0x12 => {
				self.write(self.get_de(), self.a);
				self.cycles_elapsed += 1;
			},
			/* inc de */ 0x13 => {
				self.set_de(u16::wrapping_add(self.get_de(), 1));
				self.cycles_elapsed += 1;
			},
			/* inc d */ 0x14 => {
				inc_r8(&mut self.d, &mut self.f);
			},
			/* dec d */ 0x15 => {
				dec_r8(&mut self.d, &mut self.f);
			},
			/* ld d, u8 */ 0x16 => {
				self.d = self.read_pc();
			},
			/* rla */ 0x17 => {
				rl_r8(&mut self.a, &mut self.f);
				self.f.set_z(false);
			},
			/* jr u8 */ 0x18 => { jr_cc(true, self); },
			/* add hl, de */ 0x19 => {
				add_hl_r16(self.get_de(), self);
			},
			/* ld a, [de] */ 0x1A => {
				self.a = self.read(self.get_de());
				self.cycles_elapsed += 1;
			},
			/* dec de */ 0x1B => {
				self.set_de(u16::wrapping_sub(self.get_de(), 1));
				self.cycles_elapsed += 1;
			},
			/* inc e */ 0x1C => {
				inc_r8(&mut self.e, &mut self.f);
			},
			/* dec e */ 0x1D => {
				dec_r8(&mut self.e, &mut self.f);
			},
			/* ld e, u8 */ 0x1E => {
				self.e = self.read_pc();
			},
			/* rra */ 0x1F => {
				rr_r8(&mut self.a, &mut self.f);
				self.f.set_z(false);
			},
			/* jr nz */ 0x20 => { jr_cc(!self.f.get_z(), self); },
			/* ld hl, u16 */ 0x21 => {
				self.l = self.read_pc();
				self.h = self.read_pc();
			},
			/* ld [hli], a */ 0x22 => {
				self.write(self.get_hl(), self.a);
				self.set_hl(u16::wrapping_add(self.get_hl(), 1));
				self.cycles_elapsed += 1;
			},
			/* inc hl */ 0x23 => {
				self.set_hl(u16::wrapping_add(self.get_hl(), 1));
				self.cycles_elapsed += 1;
			},
			/* inc h */ 0x24 => {
				inc_r8(&mut self.h, &mut self.f);
			},
			/* dec h */ 0x25 => {
				dec_r8(&mut self.h, &mut self.f);
			},
			/* ld h, u8 */ 0x26 => {
				self.h = self.read_pc();
			},
			/* daa */ 0x27 => {
				if !self.f.get_n() && self.a >= 0x9A { self.f.set_c(true); }
				if !self.f.get_n() && self.a & 0xF >= 0xA { self.f.set_h(true); }
				let adjustment = if self.f.get_h() { 0x6 } else { 0 } | if self.f.get_c() { 0x60 } else { 0 };
				if self.f.get_n() {
					self.a = u8::wrapping_sub(self.a, adjustment);
				} else {
					self.a = u8::wrapping_add(self.a, adjustment);
				}
				self.f.set_z(self.a == 0);
				self.f.set_h(false);
			},
			/* jr z */ 0x28 => { jr_cc(self.f.get_z(), self); },
			/* add hl, hl */ 0x29 => {
				add_hl_r16(self.get_hl(), self);
			},
			/* ld a, [hli] */ 0x2A => {
				self.a = self.read(self.get_hl());
				self.set_hl(u16::wrapping_add(self.get_hl(), 1));
				self.cycles_elapsed += 1;
			},
			/* dec hl */ 0x2B => {
				self.set_hl(u16::wrapping_sub(self.get_hl(), 1));
				self.cycles_elapsed += 1;
			},
			/* inc l */ 0x2C => {
				inc_r8(&mut self.l, &mut self.f);
			},
			/* dec l */ 0x2D => {
				dec_r8(&mut self.l, &mut self.f);
			},
			/* ld l, u8 */ 0x2E => {
				self.l = self.read_pc();
			},
			/* cpl */ 0x2F => {
				self.a = !self.a;
			},
			/* jr nc */ 0x30 => { jr_cc(!self.f.get_c(), self); },
			/* ld sp, u16 */ 0x31 => {
				self.sp = self.read_pc() as u16 | (self.read_pc() as u16) << 8;
			},
			/* ld [hld], a */ 0x32 => {
				self.write(self.get_hl(), self.a);
				self.set_hl(u16::wrapping_sub(self.get_hl(), 1));
				self.cycles_elapsed += 1;
			},
			/* inc sp */ 0x33 => {
				self.sp = u16::wrapping_add(self.sp, 1);
				self.cycles_elapsed += 1;
			},
			/* inc [hl] */ 0x34 => {
				let mut value = self.read(self.get_hl());
				inc_r8(&mut value, &mut self.f);
				self.write(self.get_hl(), value);
				self.cycles_elapsed += 2;
			},
			/* dec [hl] */ 0x35 => {
				let mut value = self.read(self.get_hl());
				dec_r8(&mut value, &mut self.f);
				self.write(self.get_hl(), value);
				self.cycles_elapsed += 2;
			},
			/* ld [hl], u8 */ 0x36 => {
				let value = self.read_pc();
				self.write(self.get_hl(), value);
				self.cycles_elapsed += 1;
			},
			/* scf */ 0x37 => {
				self.f.set_n(false);
				self.f.set_h(false);
				self.f.set_c(true);
			},
			/* jr c */ 0x38 => { jr_cc(self.f.get_c(), self); },
			/* add hl, sp */ 0x39 => {
				add_hl_r16(self.sp, self);
			},
			/* ld a, [hld] */ 0x3A => {
				self.a = self.read(self.get_hl());
				self.set_hl(u16::wrapping_sub(self.get_hl(), 1));
				self.cycles_elapsed += 1;
			},
			/* dec sp */ 0x3B => {
				self.sp = u16::wrapping_sub(self.sp, 1);
				self.cycles_elapsed += 1;
			},
			/* inc a */ 0x3C => {
				inc_r8(&mut self.a, &mut self.f);
			},
			/* dec a */ 0x3D => {
				dec_r8(&mut self.a, &mut self.f);
			},
			/* ld a, u8 */ 0x3E => {
				self.a = self.read_pc();
			},
			/* ccf */ 0x3F => {
				self.f.set_n(false);
				self.f.set_h(false);
				self.f.set_c(!self.f.get_c());
			},
			/* ld b family */
			0x40 => { return TickResult::Break; },
			0x41 => { self.b = self.c; },
			0x42 => { self.b = self.d; },
			0x43 => { self.b = self.e; },
			0x44 => { self.b = self.h; },
			0x45 => { self.b = self.l; },
			0x46 => {
				self.b = self.read(self.get_hl());
				self.cycles_elapsed += 1;
			},
			0x47 => { self.b = self.a; },
			/* ld c family */
			0x48 => { self.c = self.b; },
			0x49 => { self.c = self.c; },
			0x4A => { self.c = self.d; },
			0x4B => { self.c = self.e; },
			0x4C => { self.c = self.h; },
			0x4D => { self.c = self.l; },
			0x4E => {
				self.c = self.read(self.get_hl());
				self.cycles_elapsed += 1;
			},
			0x4F => { self.c = self.a; },
			/* ld d family */
			0x50 => { self.d = self.b; },
			0x51 => { self.d = self.c; },
			0x52 => { return TickResult::Debug; },
			0x53 => { self.d = self.e; },
			0x54 => { self.d = self.h; },
			0x55 => { self.d = self.l; },
			0x56 => {
				self.d = self.read(self.get_hl());
				self.cycles_elapsed += 1;
			},
			0x57 => { self.d = self.a; },
			/* ld e family */
			0x58 => { self.e = self.b; },
			0x59 => { self.e = self.c; },
			0x5A => { self.e = self.d; },
			0x5B => { self.e = self.e; },
			0x5C => { self.e = self.h; },
			0x5D => { self.e = self.l; },
			0x5E => {
				self.e = self.read(self.get_hl());
				self.cycles_elapsed += 1;
			},
			0x5F => { self.e = self.a; },
			/* ld h family */
			0x60 => { self.h = self.b; },
			0x61 => { self.h = self.c; },
			0x62 => { self.h = self.d; },
			0x63 => { self.h = self.e; },
			0x64 => { self.h = self.h; },
			0x65 => { self.h = self.l; },
			0x66 => {
				self.h = self.read(self.get_hl());
				self.cycles_elapsed += 1;
			},
			0x67 => { self.h = self.a; },
			/* ld l family */
			0x68 => { self.l = self.b; },
			0x69 => { self.l = self.c; },
			0x6A => { self.l = self.d; },
			0x6B => { self.l = self.e; },
			0x6C => { self.l = self.h; },
			0x6D => { self.l = self.l; },
			0x6E => {
				self.l = self.read(self.get_hl());
				self.cycles_elapsed += 1;
			},
			0x6F => { self.l = self.a; },
			/* ld [hl] family */
			0x70 => { self.write(self.get_hl(), self.b); },
			0x71 => { self.write(self.get_hl(), self.c); },
			0x72 => { self.write(self.get_hl(), self.d); },
			0x73 => { self.write(self.get_hl(), self.e); },
			0x74 => { self.write(self.get_hl(), self.h); },
			0x75 => { self.write(self.get_hl(), self.l); },
			/* halt */
			0x76 => { return TickResult::Halt; },
			/* ld [hl], a */ 0x77 => {
				self.write(self.get_hl(), self.a);
				self.cycles_elapsed += 1;
			},
			/* ld a family */
			0x78 => { self.a = self.b; },
			0x79 => { self.a = self.c; },
			0x7A => { self.a = self.d; },
			0x7B => { self.a = self.e; },
			0x7C => { self.a = self.h; },
			0x7D => { self.a = self.l; },
			0x7E => {
				self.a = self.read(self.get_hl());
				self.cycles_elapsed += 1;
			},
			0x7F => { self.a = self.a; },
			/* add family */
			0x80 => { add_a_r8(self.b, self); },
			0x81 => { add_a_r8(self.c, self); },
			0x82 => { add_a_r8(self.d, self); },
			0x83 => { add_a_r8(self.e, self); },
			0x84 => { add_a_r8(self.h, self); },
			0x85 => { add_a_r8(self.l, self); },
			0x86 => { add_a_r8(self.read(self.get_hl()), self); self.cycles_elapsed += 1; }
			0x87 => { add_a_r8(self.a, self); },
			/* adc family */
			0x88 => { adc_a_r8(self.b, self); },
			0x89 => { adc_a_r8(self.c, self); },
			0x8A => { adc_a_r8(self.d, self); },
			0x8B => { adc_a_r8(self.e, self); },
			0x8C => { adc_a_r8(self.h, self); },
			0x8D => { adc_a_r8(self.l, self); },
			0x8E => { adc_a_r8(self.read(self.get_hl()), self); self.cycles_elapsed += 1; }
			0x8F => { adc_a_r8(self.a, self); },
			/* sub family */
			0x90 => { sub_a_r8(self.b, self); },
			0x91 => { sub_a_r8(self.c, self); },
			0x92 => { sub_a_r8(self.d, self); },
			0x93 => { sub_a_r8(self.e, self); },
			0x94 => { sub_a_r8(self.h, self); },
			0x95 => { sub_a_r8(self.l, self); },
			0x96 => { sub_a_r8(self.read(self.get_hl()), self); self.cycles_elapsed += 1; }
			0x97 => { sub_a_r8(self.a, self); },
			/* sbc family */
			0x98 => { sbc_a_r8(self.b, self); },
			0x99 => { sbc_a_r8(self.c, self); },
			0x9A => { sbc_a_r8(self.d, self); },
			0x9B => { sbc_a_r8(self.e, self); },
			0x9C => { sbc_a_r8(self.h, self); },
			0x9D => { sbc_a_r8(self.l, self); },
			0x9E => { sbc_a_r8(self.read(self.get_hl()), self); self.cycles_elapsed += 1; }
			0x9F => { sbc_a_r8(self.a, self); },
			/* and family */
			0xA0 => { and_a_r8(self.b, self); },
			0xA1 => { and_a_r8(self.c, self); },
			0xA2 => { and_a_r8(self.d, self); },
			0xA3 => { and_a_r8(self.e, self); },
			0xA4 => { and_a_r8(self.h, self); },
			0xA5 => { and_a_r8(self.l, self); },
			0xA6 => { and_a_r8(self.read(self.get_hl()), self); self.cycles_elapsed += 1; }
			0xA7 => { and_a_r8(self.a, self); },
			/* xor family */
			0xA8 => { xor_a_r8(self.b, self); },
			0xA9 => { xor_a_r8(self.c, self); },
			0xAA => { xor_a_r8(self.d, self); },
			0xAB => { xor_a_r8(self.e, self); },
			0xAC => { xor_a_r8(self.h, self); },
			0xAD => { xor_a_r8(self.l, self); },
			0xAE => { xor_a_r8(self.read(self.get_hl()), self); self.cycles_elapsed += 1; }
			0xAF => { xor_a_r8(self.a, self); },
			/* or family */
			0xB0 => { or_a_r8(self.b, self); },
			0xB1 => { or_a_r8(self.c, self); },
			0xB2 => { or_a_r8(self.d, self); },
			0xB3 => { or_a_r8(self.e, self); },
			0xB4 => { or_a_r8(self.h, self); },
			0xB5 => { or_a_r8(self.l, self); },
			0xB6 => { or_a_r8(self.read(self.get_hl()), self); self.cycles_elapsed += 1; }
			0xB7 => { or_a_r8(self.a, self); },
			/* cp family */
			0xB8 => { cp_a_r8(self.b, self); },
			0xB9 => { cp_a_r8(self.c, self); },
			0xBA => { cp_a_r8(self.d, self); },
			0xBB => { cp_a_r8(self.e, self); },
			0xBC => { cp_a_r8(self.h, self); },
			0xBD => { cp_a_r8(self.l, self); },
			0xBE => { cp_a_r8(self.read(self.get_hl()), self); self.cycles_elapsed += 1; }
			0xBF => { cp_a_r8(self.a, self); },
			/* ret nz */
			0xC0 => { ret_cc(!self.f.get_z(), self); },
			/* pop bc */
			0xC1 => {
				let value = pop(self);
				self.set_bc(value);
			},
			/* jp nz */
			0xC2 => { jp_cc(!self.f.get_z(), self); },
			/* jp */
			0xC3 => { jp_cc(true, self); },
			/* call nz */
			0xC4 => { call_cc(!self.f.get_z(), self); },
			/* push bc */
			0xC5 => { push(self.get_bc(), self); },
			/* add a, u8 */
			0xC6 => { add_a_r8(self.read_pc(), self) },
			/* rst 0 */
			0xC7 => {
				push(self.pc, self);
				self.pc = 0x0000;
				self.cycles_elapsed += 1;
			},
			/* ret z */
			0xC8 => { ret_cc(self.f.get_z(), self); },
			/* ret */
			0xC9 => { ret_cc(true, self); },
			/* jp z */
			0xCA => { jp_cc(self.f.get_z(), self); },
			/* prefix byte */
			0xCB => { match self.read_pc() {
				/* rlc r8 */
				0x00 => { rlc_r8(&mut self.b, &mut self.f); },
				0x01 => { rlc_r8(&mut self.c, &mut self.f); },
				0x02 => { rlc_r8(&mut self.d, &mut self.f); },
				0x03 => { rlc_r8(&mut self.e, &mut self.f); },
				0x04 => { rlc_r8(&mut self.h, &mut self.f); },
				0x05 => { rlc_r8(&mut self.l, &mut self.f); },
				0x06 => {
					let mut value = self.address_space.read(self.get_hl());
					rlc_r8(&mut value, &mut self.f);
					self.address_space.write(self.get_hl(), value);
				},
				0x07 => { rlc_r8(&mut self.a, &mut self.f); },
				/* rrc r8 */
				0x08 => { rrc_r8(&mut self.b, &mut self.f); },
				0x09 => { rrc_r8(&mut self.c, &mut self.f); },
				0x0A => { rrc_r8(&mut self.d, &mut self.f); },
				0x0B => { rrc_r8(&mut self.e, &mut self.f); },
				0x0C => { rrc_r8(&mut self.h, &mut self.f); },
				0x0D => { rrc_r8(&mut self.l, &mut self.f); },
				0x0E => {
					let mut value = self.address_space.read(self.get_hl());
					rrc_r8(&mut value, &mut self.f);
					self.address_space.write(self.get_hl(), value);
				},
				0x0F => { rrc_r8(&mut self.a, &mut self.f); },
				/* rl r8 */
				0x10 => { rl_r8(&mut self.b, &mut self.f); },
				0x11 => { rl_r8(&mut self.c, &mut self.f); },
				0x12 => { rl_r8(&mut self.d, &mut self.f); },
				0x13 => { rl_r8(&mut self.e, &mut self.f); },
				0x14 => { rl_r8(&mut self.h, &mut self.f); },
				0x15 => { rl_r8(&mut self.l, &mut self.f); },
				0x16 => {
					let mut value = self.address_space.read(self.get_hl());
					rl_r8(&mut value, &mut self.f);
					self.address_space.write(self.get_hl(), value);
				},
				0x17 => { rl_r8(&mut self.a, &mut self.f); },
				/* rr r8 */
				0x18 => { rr_r8(&mut self.b, &mut self.f); },
				0x19 => { rr_r8(&mut self.c, &mut self.f); },
				0x1A => { rr_r8(&mut self.d, &mut self.f); },
				0x1B => { rr_r8(&mut self.e, &mut self.f); },
				0x1C => { rr_r8(&mut self.h, &mut self.f); },
				0x1D => { rr_r8(&mut self.l, &mut self.f); },
				0x1E => {
					let mut value = self.address_space.read(self.get_hl());
					rr_r8(&mut value, &mut self.f);
					self.address_space.write(self.get_hl(), value);
				},
				0x1F => { rr_r8(&mut self.a, &mut self.f); },
				/* sla r8 */
				0x20 => { sla_r8(self.b, &mut self.f); },
				0x21 => { sla_r8(self.c, &mut self.f); },
				0x22 => { sla_r8(self.d, &mut self.f); },
				0x23 => { sla_r8(self.e, &mut self.f); },
				0x24 => { sla_r8(self.h, &mut self.f); },
				0x25 => { sla_r8(self.l, &mut self.f); },
				0x26 => {
					self.address_space.write(self.get_hl(), sla_r8(self.address_space.read(self.get_hl()), &mut self.f));
					self.cycles_elapsed += 1;
				},
				0x27 => { sla_r8(self.a, &mut self.f); },
				/* sra r8 */
				0x28 => { sra_r8(self.b, &mut self.f); },
				0x29 => { sra_r8(self.c, &mut self.f); },
				0x2A => { sra_r8(self.d, &mut self.f); },
				0x2B => { sra_r8(self.e, &mut self.f); },
				0x2C => { sra_r8(self.h, &mut self.f); },
				0x2D => { sra_r8(self.l, &mut self.f); },
				0x2E => {
					self.address_space.write(self.get_hl(), sra_r8(self.address_space.read(self.get_hl()), &mut self.f));
					self.cycles_elapsed += 1;
				},
				0x2F => { sra_r8(self.a, &mut self.f); },
				/* swap r8 */
				0x30 => { self.b = swap_r8(self.b, &mut self.f); },
				0x31 => { self.c = swap_r8(self.c, &mut self.f); },
				0x32 => { self.d = swap_r8(self.d, &mut self.f); },
				0x33 => { self.e = swap_r8(self.e, &mut self.f); },
				0x34 => { self.h = swap_r8(self.h, &mut self.f); },
				0x35 => { self.l = swap_r8(self.l, &mut self.f); },
				0x36 => {
					self.address_space.write(self.get_hl(), swap_r8(self.address_space.read(self.get_hl()), &mut self.f));
					self.cycles_elapsed += 1;
				},
				0x37 => { self.a = swap_r8(self.a, &mut self.f); },
				/* srl r8 */
				0x38 => { self.b = srl_r8(self.b, &mut self.f); },
				0x39 => { self.c = srl_r8(self.c, &mut self.f); },
				0x3A => { self.d = srl_r8(self.d, &mut self.f); },
				0x3B => { self.e = srl_r8(self.e, &mut self.f); },
				0x3C => { self.h = srl_r8(self.h, &mut self.f); },
				0x3D => { self.l = srl_r8(self.l, &mut self.f); },
				0x3E => {
					self.address_space.write(self.get_hl(), srl_r8(self.address_space.read(self.get_hl()), &mut self.f));
					self.cycles_elapsed += 1;
				},
				0x3F => { self.a = srl_r8(self.a, &mut self.f); },
				/* bit r8 */
				0x40 => { bit_r8(self.b, 0, &mut self.f); },
				0x41 => { bit_r8(self.c, 0, &mut self.f); },
				0x42 => { bit_r8(self.d, 0, &mut self.f); },
				0x43 => { bit_r8(self.e, 0, &mut self.f); },
				0x44 => { bit_r8(self.h, 0, &mut self.f); },
				0x45 => { bit_r8(self.l, 0, &mut self.f); },
				0x46 => {
					bit_r8(self.address_space.read(self.get_hl()), 0, &mut self.f);
					self.cycles_elapsed += 1;
				},
				0x47 => { bit_r8(self.a, 0, &mut self.f); },
				0x48 => { bit_r8(self.b, 1, &mut self.f); },
				0x49 => { bit_r8(self.c, 1, &mut self.f); },
				0x4A => { bit_r8(self.d, 1, &mut self.f); },
				0x4B => { bit_r8(self.e, 1, &mut self.f); },
				0x4C => { bit_r8(self.h, 1, &mut self.f); },
				0x4D => { bit_r8(self.l, 1, &mut self.f); },
				0x4E => {
					bit_r8(self.address_space.read(self.get_hl()), 1, &mut self.f);
					self.cycles_elapsed += 1;
				},
				0x4F => { bit_r8(self.a, 1, &mut self.f); },
				0x50 => { bit_r8(self.b, 2, &mut self.f); },
				0x51 => { bit_r8(self.c, 2, &mut self.f); },
				0x52 => { bit_r8(self.d, 2, &mut self.f); },
				0x53 => { bit_r8(self.e, 2, &mut self.f); },
				0x54 => { bit_r8(self.h, 2, &mut self.f); },
				0x55 => { bit_r8(self.l, 2, &mut self.f); },
				0x56 => {
					bit_r8(self.address_space.read(self.get_hl()), 2, &mut self.f);
					self.cycles_elapsed += 1;
				},
				0x57 => { bit_r8(self.a, 2, &mut self.f); },
				0x58 => { bit_r8(self.b, 3, &mut self.f); },
				0x59 => { bit_r8(self.c, 3, &mut self.f); },
				0x5A => { bit_r8(self.d, 3, &mut self.f); },
				0x5B => { bit_r8(self.e, 3, &mut self.f); },
				0x5C => { bit_r8(self.h, 3, &mut self.f); },
				0x5D => { bit_r8(self.l, 3, &mut self.f); },
				0x5E => {
					bit_r8(self.address_space.read(self.get_hl()), 3, &mut self.f);
					self.cycles_elapsed += 1;
				},
				0x5F => { bit_r8(self.a, 3, &mut self.f); },
				0x60 => { bit_r8(self.b, 4, &mut self.f); },
				0x61 => { bit_r8(self.c, 4, &mut self.f); },
				0x62 => { bit_r8(self.d, 4, &mut self.f); },
				0x63 => { bit_r8(self.e, 4, &mut self.f); },
				0x64 => { bit_r8(self.h, 4, &mut self.f); },
				0x65 => { bit_r8(self.l, 4, &mut self.f); },
				0x66 => {
					bit_r8(self.address_space.read(self.get_hl()), 4, &mut self.f);
					self.cycles_elapsed += 1;
				},
				0x67 => { bit_r8(self.a, 4, &mut self.f); },
				0x68 => { bit_r8(self.b, 5, &mut self.f); },
				0x69 => { bit_r8(self.c, 5, &mut self.f); },
				0x6A => { bit_r8(self.d, 5, &mut self.f); },
				0x6B => { bit_r8(self.e, 5, &mut self.f); },
				0x6C => { bit_r8(self.h, 5, &mut self.f); },
				0x6D => { bit_r8(self.l, 5, &mut self.f); },
				0x6E => {
					bit_r8(self.address_space.read(self.get_hl()), 5, &mut self.f);
					self.cycles_elapsed += 1;
				},
				0x6F => { bit_r8(self.a, 5, &mut self.f); },
				0x70 => { bit_r8(self.b, 6, &mut self.f); },
				0x71 => { bit_r8(self.c, 6, &mut self.f); },
				0x72 => { bit_r8(self.d, 6, &mut self.f); },
				0x73 => { bit_r8(self.e, 6, &mut self.f); },
				0x74 => { bit_r8(self.h, 6, &mut self.f); },
				0x75 => { bit_r8(self.l, 6, &mut self.f); },
				0x76 => {
					bit_r8(self.address_space.read(self.get_hl()), 6, &mut self.f);
					self.cycles_elapsed += 1;
				},
				0x77 => { bit_r8(self.a, 6, &mut self.f); },
				0x78 => { bit_r8(self.b, 7, &mut self.f); },
				0x79 => { bit_r8(self.c, 7, &mut self.f); },
				0x7A => { bit_r8(self.d, 7, &mut self.f); },
				0x7B => { bit_r8(self.e, 7, &mut self.f); },
				0x7C => { bit_r8(self.h, 7, &mut self.f); },
				0x7D => { bit_r8(self.l, 7, &mut self.f); },
				0x7E => {
					bit_r8(self.address_space.read(self.get_hl()), 7, &mut self.f);
					self.cycles_elapsed += 1;
				},
				0x7F => { bit_r8(self.a, 7, &mut self.f); },
				/* res r8 */
				0x80 => { self.b = res_r8(self.b, 0); },
				0x81 => { self.c = res_r8(self.c, 0); },
				0x82 => { self.d = res_r8(self.d, 0); },
				0x83 => { self.e = res_r8(self.e, 0); },
				0x84 => { self.h = res_r8(self.h, 0); },
				0x85 => { self.l = res_r8(self.l, 0); },
				0x86 => {
					self.address_space.write(self.get_hl(), res_r8(self.address_space.read(self.get_hl()), 0));
					self.cycles_elapsed += 1;
				},
				0x87 => { self.a = res_r8(self.a, 0); },
				0x88 => { self.b = res_r8(self.b, 1); },
				0x89 => { self.c = res_r8(self.c, 1); },
				0x8A => { self.d = res_r8(self.d, 1); },
				0x8B => { self.e = res_r8(self.e, 1); },
				0x8C => { self.h = res_r8(self.h, 1); },
				0x8D => { self.l = res_r8(self.l, 1); },
				0x8E => {
					self.address_space.write(self.get_hl(), res_r8(self.address_space.read(self.get_hl()), 1));
					self.cycles_elapsed += 1;
				},
				0x8F => { self.a = res_r8(self.a, 1); },
				0x90 => { self.b = res_r8(self.b, 2); },
				0x91 => { self.c = res_r8(self.c, 2); },
				0x92 => { self.d = res_r8(self.d, 2); },
				0x93 => { self.e = res_r8(self.e, 2); },
				0x94 => { self.h = res_r8(self.h, 2); },
				0x95 => { self.l = res_r8(self.l, 2); },
				0x96 => {
					self.address_space.write(self.get_hl(), res_r8(self.address_space.read(self.get_hl()), 2));
					self.cycles_elapsed += 1;
				},
				0x97 => { self.a = res_r8(self.a, 2); },
				0x98 => { self.b = res_r8(self.b, 3); },
				0x99 => { self.c = res_r8(self.c, 3); },
				0x9A => { self.d = res_r8(self.d, 3); },
				0x9B => { self.e = res_r8(self.e, 3); },
				0x9C => { self.h = res_r8(self.h, 3); },
				0x9D => { self.l = res_r8(self.l, 3); },
				0x9E => {
					self.address_space.write(self.get_hl(), res_r8(self.address_space.read(self.get_hl()), 3));
					self.cycles_elapsed += 1;
				},
				0x9F => { self.a = res_r8(self.a, 3); },
				0xA0 => { self.b = res_r8(self.b, 4); },
				0xA1 => { self.c = res_r8(self.c, 4); },
				0xA2 => { self.d = res_r8(self.d, 4); },
				0xA3 => { self.e = res_r8(self.e, 4); },
				0xA4 => { self.h = res_r8(self.h, 4); },
				0xA5 => { self.l = res_r8(self.l, 4); },
				0xA6 => {
					self.address_space.write(self.get_hl(), res_r8(self.address_space.read(self.get_hl()), 4));
					self.cycles_elapsed += 1;
				},
				0xA7 => { self.a = res_r8(self.a, 4); },
				0xA8 => { self.b = res_r8(self.b, 5); },
				0xA9 => { self.c = res_r8(self.c, 5); },
				0xAA => { self.d = res_r8(self.d, 5); },
				0xAB => { self.e = res_r8(self.e, 5); },
				0xAC => { self.h = res_r8(self.h, 5); },
				0xAD => { self.l = res_r8(self.l, 5); },
				0xAE => {
					self.address_space.write(self.get_hl(), res_r8(self.address_space.read(self.get_hl()), 5));
					self.cycles_elapsed += 1;
				},
				0xAF => { self.a = res_r8(self.a, 5); },
				0xB0 => { self.b = res_r8(self.b, 6); },
				0xB1 => { self.c = res_r8(self.c, 6); },
				0xB2 => { self.d = res_r8(self.d, 6); },
				0xB3 => { self.e = res_r8(self.e, 6); },
				0xB4 => { self.h = res_r8(self.h, 6); },
				0xB5 => { self.l = res_r8(self.l, 6); },
				0xB6 => {
					self.address_space.write(self.get_hl(), res_r8(self.address_space.read(self.get_hl()), 6));
					self.cycles_elapsed += 1;
				},
				0xB7 => { self.a = res_r8(self.a, 6); },
				0xB8 => { self.b = res_r8(self.b, 7); },
				0xB9 => { self.c = res_r8(self.c, 7); },
				0xBA => { self.d = res_r8(self.d, 7); },
				0xBB => { self.e = res_r8(self.e, 7); },
				0xBC => { self.h = res_r8(self.h, 7); },
				0xBD => { self.l = res_r8(self.l, 7); },
				0xBE => {
					self.address_space.write(self.get_hl(), res_r8(self.address_space.read(self.get_hl()), 7));
					self.cycles_elapsed += 1;
				},
				0xBF => { self.a = res_r8(self.a, 7); },
				/* set r8 */
				0xC0 => { self.b = set_r8(self.b, 0); },
				0xC1 => { self.c = set_r8(self.c, 0); },
				0xC2 => { self.d = set_r8(self.d, 0); },
				0xC3 => { self.e = set_r8(self.e, 0); },
				0xC4 => { self.h = set_r8(self.h, 0); },
				0xC5 => { self.l = set_r8(self.l, 0); },
				0xC6 => {
					self.address_space.write(self.get_hl(), set_r8(self.address_space.read(self.get_hl()), 0));
					self.cycles_elapsed += 1;
				},
				0xC7 => { self.a = set_r8(self.a, 0); },
				0xC8 => { self.b = set_r8(self.b, 1); },
				0xC9 => { self.c = set_r8(self.c, 1); },
				0xCA => { self.d = set_r8(self.d, 1); },
				0xCB => { self.e = set_r8(self.e, 1); },
				0xCC => { self.h = set_r8(self.h, 1); },
				0xCD => { self.l = set_r8(self.l, 1); },
				0xCE => {
					self.address_space.write(self.get_hl(), set_r8(self.address_space.read(self.get_hl()), 1));
					self.cycles_elapsed += 1;
				},
				0xCF => { self.a = set_r8(self.a, 1); },
				0xD0 => { self.b = set_r8(self.b, 2); },
				0xD1 => { self.c = set_r8(self.c, 2); },
				0xD2 => { self.d = set_r8(self.d, 2); },
				0xD3 => { self.e = set_r8(self.e, 2); },
				0xD4 => { self.h = set_r8(self.h, 2); },
				0xD5 => { self.l = set_r8(self.l, 2); },
				0xD6 => {
					self.address_space.write(self.get_hl(), set_r8(self.address_space.read(self.get_hl()), 2));
					self.cycles_elapsed += 1;
				},
				0xD7 => { self.a = set_r8(self.a, 2); },
				0xD8 => { self.b = set_r8(self.b, 3); },
				0xD9 => { self.c = set_r8(self.c, 3); },
				0xDA => { self.d = set_r8(self.d, 3); },
				0xDB => { self.e = set_r8(self.e, 3); },
				0xDC => { self.h = set_r8(self.h, 3); },
				0xDD => { self.l = set_r8(self.l, 3); },
				0xDE => {
					self.address_space.write(self.get_hl(), set_r8(self.address_space.read(self.get_hl()), 3));
					self.cycles_elapsed += 1;
				},
				0xDF => { self.a = set_r8(self.a, 3); },
				0xE0 => { self.b = set_r8(self.b, 4); },
				0xE1 => { self.c = set_r8(self.c, 4); },
				0xE2 => { self.d = set_r8(self.d, 4); },
				0xE3 => { self.e = set_r8(self.e, 4); },
				0xE4 => { self.h = set_r8(self.h, 4); },
				0xE5 => { self.l = set_r8(self.l, 4); },
				0xE6 => {
					self.address_space.write(self.get_hl(), set_r8(self.address_space.read(self.get_hl()), 4));
					self.cycles_elapsed += 1;
				},
				0xE7 => { self.a = set_r8(self.a, 4); },
				0xE8 => { self.b = set_r8(self.b, 5); },
				0xE9 => { self.c = set_r8(self.c, 5); },
				0xEA => { self.d = set_r8(self.d, 5); },
				0xEB => { self.e = set_r8(self.e, 5); },
				0xEC => { self.h = set_r8(self.h, 5); },
				0xED => { self.l = set_r8(self.l, 5); },
				0xEE => {
					self.address_space.write(self.get_hl(), set_r8(self.address_space.read(self.get_hl()), 5));
					self.cycles_elapsed += 1;
				},
				0xEF => { self.a = set_r8(self.a, 5); },
				0xF0 => { self.b = set_r8(self.b, 6); },
				0xF1 => { self.c = set_r8(self.c, 6); },
				0xF2 => { self.d = set_r8(self.d, 6); },
				0xF3 => { self.e = set_r8(self.e, 6); },
				0xF4 => { self.h = set_r8(self.h, 6); },
				0xF5 => { self.l = set_r8(self.l, 6); },
				0xF6 => {
					self.address_space.write(self.get_hl(), set_r8(self.address_space.read(self.get_hl()), 6));
					self.cycles_elapsed += 1;
				},
				0xF7 => { self.a = set_r8(self.a, 6); },
				0xF8 => { self.b = set_r8(self.b, 7); },
				0xF9 => { self.c = set_r8(self.c, 7); },
				0xFA => { self.d = set_r8(self.d, 7); },
				0xFB => { self.e = set_r8(self.e, 7); },
				0xFC => { self.h = set_r8(self.h, 7); },
				0xFD => { self.l = set_r8(self.l, 7); },
				0xFE => {
					self.address_space.write(self.get_hl(), set_r8(self.address_space.read(self.get_hl()), 7));
					self.cycles_elapsed += 1;
				},
				0xFF => { self.a = set_r8(self.a, 7); },
			} },
			/* call z */
			0xCC => { call_cc(self.f.get_z(), self); },
			/* call */
			0xCD => { call_cc(true, self); },
			/* adc a, u8 */
			0xCE => { adc_a_r8(self.read_pc(), self)},
			/* rst 8 */
			0xCF => {
				push(self.pc, self);
				self.pc = 0x0008;
				self.cycles_elapsed += 1;
			},
			/* ret nc */
			0xD0 => { ret_cc(!self.f.get_c(), self); },
			/* pop de */
			0xD1 => {
				let value = pop(self);
				self.set_de(value);
			},
			/* jp nc */
			0xD2 => { jp_cc(!self.f.get_c(), self); },
			/* invalid opcode */
			/* call nc */
			0xD4 => { call_cc(!self.f.get_c(), self); },
			/* push de */
			0xD5 => { push(self.get_de(), self); },
			/* sub a, u8 */
			0xD6 => { sub_a_r8(self.read_pc(), self) },
			/* rst 10 */
			0xD7 => {
				push(self.pc, self);
				self.pc = 0x0010;
				self.cycles_elapsed += 1;
			},
			/* ret c */
			0xD8 => { ret_cc(self.f.get_c(), self); },
			/* reti */
			0xD9 => {
				ret_cc(true, self);
				self.ei = true;
			}
			/* jp c */
			0xDA => { jp_cc(self.f.get_c(), self); },
			/* invalid opcode */
			/* call c */
			0xDC => { call_cc(self.f.get_c(), self); },
			/* invalid opcode */
			/* sbc a, u8 */
			0xDE => { sbc_a_r8(self.read_pc(), self)},
			/* rst 18 */
			0xDF => {
				push(self.pc, self);
				self.pc = 0x0018;
				self.cycles_elapsed += 1;
			},
			/* ldh [u16], a */
			0xE0 => {
				let value = self.read_pc() as u16;
				self.write(0xFF00 | value, self.a);
				self.cycles_elapsed += 1;
			},
			/* pop hl */
			0xE1 => {
				let value = pop(self);
				self.set_hl(value);
			},
			/* ldh [c], a */
			0xE2 => {
				self.write(0xFF00 | self.c as u16, self.a);
				self.cycles_elapsed += 1;
			},
			/* invalid opcode */
			/* invalid opcode */
			/* push hl */
			0xE5 => { push(self.get_hl(), self); },
			/* and a, u8 */
			0xE6 => { and_a_r8(self.read_pc(), self); },
			/* rst 20 */
			0xE7 => {
				push(self.pc, self);
				self.pc = 0x0020;
				self.cycles_elapsed += 1;
			},
			/* add sp, u8 */
			0xE8 => {
				let value = self.read_pc() as u16;
				let old_sp = self.sp;
				self.sp = u16::wrapping_add(self.sp, value);
				self.f.set_z(false);
				self.f.set_n(false);
				self.f.set_h((old_sp & 0xFFF + value & 0xFFF > 0xFFF) == true);
				self.f.set_c(old_sp > self.sp);
				self.cycles_elapsed += 2;
			},
			/* jp hl */
			0xE9 => {
				self.pc = self.get_hl();
				self.cycles_elapsed += 1;
			},
			/* ld [u16], a */
			0xEA => {
				let address = self.read_pc() as u16 | (self.read_pc() as u16) << 8;
				self.write(address, self.a);
				self.cycles_elapsed += 1;
			},
			/* invalid opcode */
			/* invalid opcode */
			/* invalid opcode */
			/* xor a, u8 */
			0xEE => { xor_a_r8(self.read_pc(), self); },
			/* rst 28 */
			0xEF => {
				push(self.pc, self);
				self.pc = 0x0028;
				self.cycles_elapsed += 1;
			},
			/* ldh a, [u16] */
			0xF0 => {
				let address = 0xFF00 | self.read_pc() as u16;
				self.a = self.read(address);
				self.cycles_elapsed += 1;
			},
			/* pop af */
			0xF1 => {
				let value = pop(self);
				self.set_af(value);
			},
			/* ldh a, [c] */
			0xF2 => {
				self.a = self.read(0xFF00 | self.c as u16);
				self.cycles_elapsed += 1;
			},
			/* di */
			0xF3 => { self.ei = false; },
			/* invalid opcode */
			/* push af */
			0xF5 => { push(self.get_af(), self); },
			/* or a, u8 */
			0xF6 => { or_a_r8(self.read_pc(), self); },
			/* rst 30 */
			0xF7 => {
				push(self.pc, self);
				self.pc = 0x0030;
				self.cycles_elapsed += 1;
			},
			/* ld hl, sp + u8 */
			0xF8 => {
				let value = self.read_pc() as u16;
				let old_sp = self.sp;
				self.set_hl(u16::wrapping_add(self.sp, value));
				self.f.set_z(false);
				self.f.set_n(false);
				self.f.set_h(((old_sp & 0xFFF) + (value & 0xFFF) > 0xFFF) == true);
				self.f.set_c(old_sp > self.get_hl());
				self.cycles_elapsed += 1;
			},
			/* jp hl */
			0xF9 => { self.sp = self.get_hl(); },
			/* ld a, [u16] */
			0xFA => {
				let address = (self.read_pc() as u16) | (self.read_pc() as u16) << 8;
				self.a = self.read(address);
				self.cycles_elapsed += 1;
			},
			/* ei */
			0xFB => { self.ei = true; },
			/* invalid opcode */
			/* invalid opcode */
			/* cp a, u8 */
			0xFE => { cp_a_r8(self.read_pc(), self); },
			/* rst 38 */
			0xFF => {
				push(self.pc, self);
				self.pc = 0x0038;
				self.cycles_elapsed += 1;
			},
			_ => panic!("Invalid opcode"),
		}

		TickResult::Ok
	}

	pub fn new(address_space: AddressSpace) -> State {
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
			// SP defaults to the top of WRAM to minimize conflicts.
			// Users should set SP to its proper address for all tests.
			sp: 0xE000, 
			ei: true,
			cycles_elapsed: 0,

			address_space,
		}
	}
}

impl fmt::Display for State {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, 
"\
a:  0x{:02x}
bc: 0x{:02x}{:02x}
de: 0x{:02x}{:02x}
hl: 0x{:02x}{:02x}
f: {}
pc: 0x{:04x}
sp: 0x{:04x}
Interrupts {}abled
Elapsed cycles: {}",
			self.a, self.b, self.c, self.d, self.e, self.h, self.l, self.f, self.pc, self.sp,
			if self.ei { "en" } else { "dis" },
			self.cycles_elapsed)
	}
}