use crate::memory::AddressSpace;
use std::{fmt, ops::BitOrAssign};

pub struct Flags {
	pub value: u8,
}

impl Flags {
	pub fn get_z(&self) -> bool {
		self.value & 0b10000000 != 0
	}
	pub fn get_n(&self) -> bool {
		self.value & 0b01000000 != 0
	}
	pub fn get_h(&self) -> bool {
		self.value & 0b00100000 != 0
	}
	pub fn get_c(&self) -> bool {
		self.value & 0b00010000 != 0
	}
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
		write!(
			f,
			"{}{}{}{}",
			if self.get_z() { 'z' } else { '-' },
			if self.get_n() { 'n' } else { '-' },
			if self.get_h() { 'h' } else { '-' },
			if self.get_c() { 'c' } else { '-' },
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

pub struct State<S: AddressSpace> {
	// Primary CPU Registers
	pub a: u8,
	pub f: Flags,
	pub b: u8,
	pub c: u8,
	pub d: u8,
	pub e: u8,
	pub h: u8,
	pub l: u8,
	pub pc: u16,
	pub sp: u16,

	pub ei: bool,

	// Total number of M-Cycles that have passed during this CPU's life.
	pub cycles_elapsed: usize,

	pub address_space: S,
}

impl<S: AddressSpace> State<S> {
	pub fn new(address_space: S) -> Self {
		Self {
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

	pub fn get_af(&self) -> u16 {
		u16::from_be_bytes([self.a, self.f.value])
	}
	pub fn get_bc(&self) -> u16 {
		u16::from_be_bytes([self.b, self.c])
	}
	pub fn get_de(&self) -> u16 {
		u16::from_be_bytes([self.d, self.e])
	}
	pub fn get_hl(&self) -> u16 {
		u16::from_be_bytes([self.h, self.l])
	}
	pub fn set_af(&mut self, value: u16) {
		[self.a, self.f.value] = value.to_be_bytes();
	}
	pub fn set_bc(&mut self, value: u16) {
		[self.b, self.c] = value.to_be_bytes();
	}
	pub fn set_de(&mut self, value: u16) {
		[self.d, self.e] = value.to_be_bytes();
	}
	pub fn set_hl(&mut self, value: u16) {
		[self.h, self.l] = value.to_be_bytes();
	}

	pub fn get_r8_by_id(&mut self, id: u8) -> u8 {
		match id {
			0 => self.b,
			1 => self.c,
			2 => self.d,
			3 => self.e,
			4 => self.h,
			5 => self.l,
			6 => {
				self.cycles_elapsed += 1;
				self.read(self.get_hl())
			}
			7 => self.a,
			_ => unreachable!(),
		}
	}
	pub fn set_r8_by_id(&mut self, id: u8, value: u8) {
		match id {
			0 => self.b = value,
			1 => self.c = value,
			2 => self.d = value,
			3 => self.e = value,
			4 => self.h = value,
			5 => self.l = value,
			6 => {
				self.cycles_elapsed += 1;
				self.write(self.get_hl(), value)
			}
			7 => self.a = value,
			_ => unreachable!(),
		}
	}

	// Passthroughs for address_space.read/write()
	pub fn read(&self, address: u16) -> u8 {
		self.address_space.read(address)
	}
	pub fn write(&mut self, address: u16, value: u8) {
		self.address_space.write(address, value);
	}

	fn read_pc(&mut self) -> u8 {
		let value = self.address_space.read(self.pc);
		self.pc = u16::wrapping_add(self.pc, 1);
		self.cycles_elapsed += 1;
		value
	}

	fn add_hl_r16(&mut self, operand: u16) {
		let value = self.get_hl().wrapping_add(operand);
		self.set_hl(self.get_hl().wrapping_add(value));
		self.f.set_n(false);
		self.f.set_h(value & 0xFFF < operand & 0xFFF);
		self.f.set_c(value > self.get_hl());
		self.cycles_elapsed += 1;
	}

	fn jr_cc(&mut self, condition: bool) {
		let offset = self.read_pc() as i8;
		if condition {
			self.pc = i16::wrapping_add(self.pc as i16, offset as i16) as u16;
			self.cycles_elapsed += 1;
		}
	}

	fn add(&mut self, operand: u8, carry_in: bool) {
		let mut r = CarryRetainer::new();
		r |= self.a.overflowing_add(operand);
		r |= r.0.overflowing_add(carry_in.into());
		// If carry is not set, it's easy: is the post-add strictly lower than the pre-add?
		// If carry is set, it's instead: is the post-add lower than the pre-add?
		// So that amounts to "is the post-add strictly lower than (the pre-add plus carry)?"
		self.f.set_h(r.0 & 0xF < (self.a & 0xF) + carry_in as u8);
		self.f.set_c(r.1);
		self.f.set_z(r.0 == 0);
		self.f.set_n(false);
		self.a = r.0;
	}

	fn sub(&mut self, operand: u8, carry_in: bool) {
		let mut r = CarryRetainer::new();
		r |= self.a.overflowing_sub(operand);
		r |= r.0.overflowing_sub(carry_in.into());
		// If carry is not set, it's easy: is the post-add strictly greater than the pre-add?
		// If carry is set, it's instead: is the post-add greater than the pre-add?
		// So that amounts to "is (the post-add plus carry) strictly lower than the pre-add?"
		self.f.set_h((r.0 & 0xF) + carry_in as u8 > self.a & 0xF);
		self.f.set_c(r.1);
		self.f.set_z(r.0 == 0);
		self.f.set_n(true);
		self.a = r.0;
	}

	fn and(&mut self, operand: u8) {
		self.a &= operand;
		self.f.set_z(self.a == 0);
		self.f.set_n(false);
		self.f.set_h(true); // No, this is not a typo.
		self.f.set_c(false);
	}

	fn xor(&mut self, operand: u8) {
		self.a ^= operand;
		self.f.set_z(self.a == 0);
		self.f.set_n(false);
		self.f.set_h(false);
		self.f.set_c(false);
	}

	fn or(&mut self, operand: u8) {
		self.a |= operand;
		self.f.set_z(self.a == 0);
		self.f.set_n(false);
		self.f.set_h(false);
		self.f.set_c(false);
	}

	fn cp(&mut self, operand: u8) {
		let (result, carry) = self.a.overflowing_sub(operand);
		self.f.set_h(result & 0xF > self.a & 0xF);
		self.f.set_c(carry);
		self.f.set_z(result == 0);
		self.f.set_n(true);
	}

	fn rlc(&mut self, reg_id: u8) {
		let value = self.get_r8_by_id(reg_id);
		let value = value.rotate_left(1);
		self.set_r8_by_id(reg_id, value);
		self.f.set_z(value == 0);
		self.f.set_c(value & 0x01 != 0);
		self.f.set_n(false);
		self.f.set_h(false);
	}

	fn rl(&mut self, reg_id: u8) {
		let value = self.get_r8_by_id(reg_id);
		let carry = value & 0x80 != 0;
		let value = value.wrapping_shl(1) | self.f.get_c() as u8;
		self.set_r8_by_id(reg_id, value);
		self.f.set_z(value == 0);
		self.f.set_c(carry);
		self.f.set_n(false);
		self.f.set_h(false);
	}

	fn rrc(&mut self, reg_id: u8) {
		let value = self.get_r8_by_id(reg_id);
		let value = value.rotate_right(1);
		self.set_r8_by_id(reg_id, value);
		self.f.set_z(value == 0);
		self.f.set_c(value & 0x80 != 0);
		self.f.set_n(false);
		self.f.set_h(false);
	}

	fn rr(&mut self, reg_id: u8) {
		let value = self.get_r8_by_id(reg_id);
		let carry = value & 0x01 != 0;
		let value = value.wrapping_shr(1) | (self.f.get_c() as u8) << 7;
		self.set_r8_by_id(reg_id, value);
		self.f.set_z(value == 0);
		self.f.set_c(carry);
		self.f.set_n(false);
		self.f.set_h(false);
	}

	fn sla(&mut self, reg_id: u8) {
		let value = self.get_r8_by_id(reg_id);
		self.set_r8_by_id(reg_id, value.wrapping_shl(1));
	}

	fn sra(&mut self, reg_id: u8) {
		let value = self.get_r8_by_id(reg_id);
		self.set_r8_by_id(reg_id, value.wrapping_shr(1) | (value & 0x80));
	}

	fn swap(&mut self, reg_id: u8) {
		let value = self.get_r8_by_id(reg_id);
		self.set_r8_by_id(reg_id, value.rotate_left(4));
	}

	fn srl(&mut self, reg_id: u8) {
		let value = self.get_r8_by_id(reg_id);
		self.set_r8_by_id(reg_id, value.wrapping_shr(1));
	}

	fn ret_cc(&mut self, condition: bool) {
		if condition {
			self.pc = self.pop();
			self.cycles_elapsed += 2; // pop already takes care of 2 extra cycles.
		} else {
			self.cycles_elapsed += 1;
		}
	}

	fn pop(&mut self) -> u16 {
		let mut result = (self.read(self.sp) as u16) << 8;
		self.sp += 1;
		result |= self.read(self.sp) as u16;
		self.sp += 1;
		self.cycles_elapsed += 2;
		result
	}

	fn jp_cc(&mut self, condition: bool) {
		if condition {
			self.pc = (self.read_pc() as u16) | (self.read_pc() as u16) << 8;
			self.cycles_elapsed += 1;
		} else {
			self.read_pc();
			self.read_pc();
		}
	}

	fn call_cc(&mut self, condition: bool) {
		if condition {
			self.push(self.pc + 2);
			self.pc = (self.read_pc() as u16) | (self.read_pc() as u16) << 8;
			self.cycles_elapsed += 1;
		} else {
			self.read_pc();
			self.read_pc();
		}
	}

	fn push(&mut self, value: u16) {
		self.sp = self.sp.wrapping_sub(1);
		self.write(self.sp, (value & 0xFF) as u8);
		self.sp = self.sp.wrapping_sub(1);
		self.write(self.sp, (value >> 8) as u8);
		self.cycles_elapsed += 3;
	}

	// Returns true upon test completion.
	pub fn tick(&mut self) -> TickResult {
		match self.read_pc() {
			/* nop */ 0x00 => {}
			/* ld bc, u16 */
			0x01 => {
				self.c = self.read_pc();
				self.b = self.read_pc();
			}
			/* ld [bc], a */
			0x02 => {
				self.write(self.get_bc(), self.a);
				self.cycles_elapsed += 1;
			}
			/* inc bc */
			0x03 => {
				self.set_bc(self.get_bc().wrapping_add(1));
				self.cycles_elapsed += 1;
			}
			/* inc r8 */
			opcode @ (0x04 | 0x0C | 0x14 | 0x1C | 0x24 | 0x2C | 0x34 | 0x3C) => {
				let reg_id = opcode >> 3;
				let value = self.get_r8_by_id(reg_id).wrapping_add(1);
				self.set_r8_by_id(reg_id, value);
				self.f.set_z(value == 0);
				self.f.set_n(false);
				self.f.set_h(value & 0xF == 0);
			}
			/* dec r8 */
			opcode @ (0x05 | 0x0D | 0x15 | 0x1D | 0x25 | 0x2D | 0x35 | 0x3D) => {
				let reg_id = opcode >> 3;
				let value = self.get_r8_by_id(reg_id).wrapping_sub(1);
				self.set_r8_by_id(reg_id, value);
				self.f.set_z(value == 0);
				self.f.set_n(false);
				self.f.set_h(value & 0xF == 0xF);
			}
			/* ld r8, u8 */
			opcode @ (0x06 | 0x0E | 0x16 | 0x1E | 0x26 | 0x2E | 0x36 | 0x3E) => {
				let value = self.read_pc();
				self.set_r8_by_id(opcode >> 3, value);
			}
			/* rlca */
			0x07 => {
				self.rlc(7);
				self.f.set_z(false);
			}
			/* ld [u16], sp */
			0x08 => {
				let pointer = u16::from_le_bytes([self.read_pc(), self.read_pc()]);
				self.write(pointer, (self.sp & 0xFF) as u8);
				self.write(pointer + 1, (self.sp >> 8) as u8);
				self.cycles_elapsed += 4;
			}
			/* add hl, bc */
			0x09 => {
				self.add_hl_r16(self.get_bc());
			}
			/* ld a, [bc] */
			0x0A => {
				self.a = self.read(self.get_bc());
				self.cycles_elapsed += 1;
			}
			/* dec bc */
			0x0B => {
				self.set_bc(self.get_bc().wrapping_sub(1));
				self.cycles_elapsed += 1;
			}
			/* rrca */
			0x0F => {
				self.rrc(7);
				self.f.set_z(false);
			}
			/* stop */
			0x10 => {
				self.read_pc();
				return TickResult::Stop;
			}
			/* ld de, u16 */
			0x11 => {
				self.e = self.read_pc();
				self.d = self.read_pc();
			}
			/* ld [de], a */
			0x12 => {
				self.write(self.get_de(), self.a);
				self.cycles_elapsed += 1;
			}
			/* inc de */
			0x13 => {
				self.set_de(self.get_de().wrapping_add(1));
				self.cycles_elapsed += 1;
			}
			/* rla */
			0x17 => {
				self.rl(7);
				self.f.set_z(false);
			}
			/* jr u8 */ 0x18 => {
				self.jr_cc(true);
			}
			/* add hl, de */
			0x19 => {
				self.add_hl_r16(self.get_de());
			}
			/* ld a, [de] */
			0x1A => {
				self.a = self.read(self.get_de());
				self.cycles_elapsed += 1;
			}
			/* dec de */
			0x1B => {
				self.set_de(self.get_de().wrapping_sub(1));
				self.cycles_elapsed += 1;
			}
			/* rra */
			0x1F => {
				self.rr(7);
				self.f.set_z(false);
			}
			/* jr nz */ 0x20 => {
				self.jr_cc(!self.f.get_z());
			}
			/* ld hl, u16 */
			0x21 => {
				self.l = self.read_pc();
				self.h = self.read_pc();
			}
			/* ld [hli], a */
			0x22 => {
				self.write(self.get_hl(), self.a);
				self.set_hl(self.get_hl().wrapping_add(1));
				self.cycles_elapsed += 1;
			}
			/* inc hl */
			0x23 => {
				self.set_hl(self.get_hl().wrapping_add(1));
				self.cycles_elapsed += 1;
			}
			/* daa */
			0x27 => {
				if !self.f.get_n() && self.a >= 0x9A {
					self.f.set_c(true);
				}
				if !self.f.get_n() && self.a & 0xF >= 0xA {
					self.f.set_h(true);
				}
				let adjustment =
					if self.f.get_h() { 0x6 } else { 0 } | if self.f.get_c() { 0x60 } else { 0 };
				if self.f.get_n() {
					self.a = self.a.wrapping_sub(adjustment);
				} else {
					self.a = self.a.wrapping_add(adjustment);
				}
				self.f.set_z(self.a == 0);
				self.f.set_h(false);
			}
			/* jr z */ 0x28 => {
				self.jr_cc(self.f.get_z());
			}
			/* add hl, hl */
			0x29 => {
				self.add_hl_r16(self.get_hl());
			}
			/* ld a, [hli] */
			0x2A => {
				self.a = self.read(self.get_hl());
				self.set_hl(self.get_hl().wrapping_add(1));
				self.cycles_elapsed += 1;
			}
			/* dec hl */
			0x2B => {
				self.set_hl(self.get_hl().wrapping_sub(1));
				self.cycles_elapsed += 1;
			}
			/* cpl */ 0x2F => {
				self.a = !self.a;
			}
			/* jr nc */ 0x30 => {
				self.jr_cc(!self.f.get_c());
			}
			/* ld sp, u16 */
			0x31 => {
				self.sp = u16::from_le_bytes([self.read_pc(), self.read_pc()]);
			}
			/* ld [hld], a */
			0x32 => {
				self.write(self.get_hl(), self.a);
				self.set_hl(self.get_hl().wrapping_sub(1));
				self.cycles_elapsed += 1;
			}
			/* inc sp */
			0x33 => {
				self.sp = self.sp.wrapping_add(1);
				self.cycles_elapsed += 1;
			}
			/* scf */
			0x37 => {
				self.f.set_n(false);
				self.f.set_h(false);
				self.f.set_c(true);
			}
			/* jr c */ 0x38 => {
				self.jr_cc(self.f.get_c());
			}
			/* add hl, sp */ 0x39 => {
				self.add_hl_r16(self.sp);
			}
			/* ld a, [hld] */
			0x3A => {
				self.a = self.read(self.get_hl());
				self.set_hl(self.get_hl().wrapping_sub(1));
				self.cycles_elapsed += 1;
			}
			/* dec sp */
			0x3B => {
				self.sp = self.sp.wrapping_sub(1);
				self.cycles_elapsed += 1;
			}
			/* ccf */
			0x3F => {
				self.f.set_n(false);
				self.f.set_h(false);
				self.f.set_c(!self.f.get_c());
			}

			/* ld b,b */
			0x40 => {
				return TickResult::Break;
			}
			/* ld d, d */
			0x52 => {
				return TickResult::Debug;
			}
			/* ld [hl], [hl] is actually halt */
			0x76 => {
				return TickResult::Halt;
			}
			/* ld r8,r8 family, special cases handled above */
			opcode @ 0x40..=0x7F => {
				let value = self.get_r8_by_id(opcode & 7);
				self.set_r8_by_id((opcode >> 3) & 7, value);
			}

			/* add a, r8 */
			opcode @ 0x80..=0x87 => {
				let value = self.get_r8_by_id(opcode & 7);
				self.add(value, false);
			}
			/* adc a, r8 */
			opcode @ 0x88..=0x8F => {
				let value = self.get_r8_by_id(opcode & 7);
				self.add(value, self.f.get_c());
			}
			/* sub a, r8 */
			opcode @ 0x90..=0x97 => {
				let value = self.get_r8_by_id(opcode & 7);
				self.sub(value, false);
			}
			/* sbc a, r8 */
			opcode @ 0x98..=0x9F => {
				let value = self.get_r8_by_id(opcode & 7);
				self.sub(value, self.f.get_c());
			}
			/* and a, r8 */
			opcode @ 0xA0..=0xA7 => {
				let value = self.get_r8_by_id(opcode & 7);
				self.and(value);
			}
			/* xor a, r8 */
			opcode @ 0xA8..=0xAF => {
				let value = self.get_r8_by_id(opcode & 7);
				self.xor(value);
			}
			/* or a, r8 */
			opcode @ 0xB0..=0xB7 => {
				let value = self.get_r8_by_id(opcode & 7);
				self.or(value);
			}
			/* cp a, r8 */
			opcode @ 0xB8..=0xBF => {
				let value = self.get_r8_by_id(opcode & 7);
				self.cp(value);
			}

			/* ret nz */
			0xC0 => {
				self.ret_cc(!self.f.get_z());
			}
			/* pop bc */
			0xC1 => {
				let value = self.pop();
				self.set_bc(value);
			}
			/* jp nz */
			0xC2 => {
				self.jp_cc(!self.f.get_z());
			}
			/* jp */
			0xC3 => {
				self.jp_cc(true);
			}
			/* call nz */
			0xC4 => {
				self.call_cc(!self.f.get_z());
			}
			/* push bc */
			0xC5 => {
				self.push(self.get_bc());
			}
			/* add a, u8 */
			0xC6 => {
				let value = self.read_pc();
				self.add(value, false)
			}
			/* rst xx */
			opcode @ (0xC7 | 0xCF | 0xD7 | 0xDF | 0xE7 | 0xEF | 0xF7 | 0xFF) => {
				self.push(self.pc);
				self.pc = (opcode & 7).into();
				self.cycles_elapsed += 1;
			}
			/* ret z */
			0xC8 => {
				self.ret_cc(self.f.get_z());
			}
			/* ret */
			0xC9 => {
				self.ret_cc(true);
			}
			/* jp z */
			0xCA => {
				self.jp_cc(self.f.get_z());
			}
			/* prefix byte */
			0xCB => {
				match self.read_pc() {
					/* rlc r8 */
					opcode @ 0x00..=0x07 => self.rlc(opcode & 7),
					/* rrc r8 */
					opcode @ 0x08..=0x0F => self.rrc(opcode & 7),
					/* rl r8 */
					opcode @ 0x10..=0x17 => self.rl(opcode & 7),
					/* rr r8 */
					opcode @ 0x18..=0x1F => self.rr(opcode & 7),
					/* sla r8 */
					opcode @ 0x20..=0x27 => self.sla(opcode & 7),
					/* sra r8 */
					opcode @ 0x28..=0x2F => self.sra(opcode & 7),
					/* swap r8 */
					opcode @ 0x30..=0x37 => self.swap(opcode & 7),
					/* srl r8 */
					opcode @ 0x38..=0x3F => self.srl(opcode & 7),
					/* bit r8 */
					opcode @ 0x40..=0x7F => {
						let value = self.get_r8_by_id(opcode & 7)
							& 1u8.overflowing_shl((opcode >> 3).into()).0;
						self.f.set_z(value != 0);
					}
					/* res n, r8 */
					opcode @ 0x80..=0xBF => {
						let mut value = self.get_r8_by_id(opcode & 7);
						value &= !1u8.overflowing_shl((opcode >> 3).into()).0; // *evil grin*
						self.set_r8_by_id(opcode & 7, value);
					}
					/* set n, r8 */
					opcode @ 0xC0..=0xFF => {
						let mut value = self.get_r8_by_id(opcode & 7);
						value |= 1u8.overflowing_shl((opcode >> 3).into()).0; // *evil grin*
						self.set_r8_by_id(opcode & 7, value);
					}
				}
			}
			/* call z */
			0xCC => {
				self.call_cc(self.f.get_z());
			}
			/* call */
			0xCD => {
				self.call_cc(true);
			}
			/* adc a, u8 */
			0xCE => {
				let value = self.read_pc();
				self.add(value, self.f.get_c());
			}
			/* ret nc */
			0xD0 => {
				self.ret_cc(!self.f.get_c());
			}
			/* pop de */
			0xD1 => {
				let value = self.pop();
				self.set_de(value);
			}
			/* jp nc */
			0xD2 => {
				self.jp_cc(!self.f.get_c());
			}
			/* invalid opcode */
			/* call nc */
			0xD4 => {
				self.call_cc(!self.f.get_c());
			}
			/* push de */
			0xD5 => {
				self.push(self.get_de());
			}
			/* sub a, u8 */
			0xD6 => {
				let value = self.read_pc();
				self.sub(value, false);
			}
			/* ret c */
			0xD8 => {
				self.ret_cc(self.f.get_c());
			}
			/* reti */
			0xD9 => {
				self.ret_cc(true);
				self.ei = true;
			}
			/* jp c */
			0xDA => {
				self.jp_cc(self.f.get_c());
			}
			/* invalid opcode */
			/* call c */
			0xDC => {
				self.call_cc(self.f.get_c());
			}
			/* invalid opcode */
			/* sbc a, u8 */
			0xDE => {
				let value = self.read_pc();
				self.sub(value, self.f.get_c());
			}
			/* ldh [u16], a */
			0xE0 => {
				let value = self.read_pc() as u16;
				self.write(0xFF00 | value, self.a);
				self.cycles_elapsed += 1;
			}
			/* pop hl */
			0xE1 => {
				let value = self.pop();
				self.set_hl(value);
			}
			/* ldh [c], a */
			0xE2 => {
				self.write(0xFF00 | self.c as u16, self.a);
				self.cycles_elapsed += 1;
			}
			/* invalid opcode */
			/* invalid opcode */
			/* push hl */
			0xE5 => {
				self.push(self.get_hl());
			}
			/* and a, u8 */
			0xE6 => {
				let value = self.read_pc();
				self.and(value);
			}
			/* add sp, u8 */
			0xE8 => {
				let value = self.read_pc() as u16;
				let old_sp = self.sp;
				self.sp = self.sp.wrapping_add(value);
				self.f.set_h((old_sp & 0xFFF) + (value & 0xFFF) > 0xFFF);
				self.f.set_c(old_sp > self.sp);
				self.f.set_z(false);
				self.f.set_n(false);
				self.cycles_elapsed += 2;
			}
			/* jp hl */
			0xE9 => {
				self.pc = self.get_hl();
				self.cycles_elapsed += 1;
			}
			/* ld [u16], a */
			0xEA => {
				let address = u16::from_le_bytes([self.read_pc(), self.read_pc()]);
				self.write(address, self.a);
				self.cycles_elapsed += 1;
			}
			/* invalid opcode */
			/* invalid opcode */
			/* invalid opcode */
			/* xor a, u8 */
			0xEE => {
				let value = self.read_pc();
				self.xor(value);
			}
			/* ldh a, [u16] */
			0xF0 => {
				let address = 0xFF00 | self.read_pc() as u16;
				self.a = self.read(address);
				self.cycles_elapsed += 1;
			}
			/* pop af */
			0xF1 => {
				let value = self.pop();
				self.set_af(value);
			}
			/* ldh a, [c] */
			0xF2 => {
				self.a = self.read(0xFF00 | self.c as u16);
				self.cycles_elapsed += 1;
			}
			/* di */
			0xF3 => {
				self.ei = false;
			}
			/* invalid opcode */
			/* push af */
			0xF5 => {
				self.push(self.get_af());
			}
			/* or a, u8 */
			0xF6 => {
				let value = self.read_pc();
				self.or(value);
			}
			/* ld hl, sp + u8 */
			0xF8 => {
				let value = self.read_pc() as u16;
				let old_sp = self.sp;
				self.set_hl(self.sp.wrapping_add(value));
				self.f.set_h((old_sp & 0xFFF) + (value & 0xFFF) > 0xFFF);
				self.f.set_c(old_sp > self.get_hl());
				self.f.set_z(false);
				self.f.set_n(false);
				self.cycles_elapsed += 1;
			}
			/* jp hl */
			0xF9 => {
				self.sp = self.get_hl();
			}
			/* ld a, [u16] */
			0xFA => {
				let address = u16::from_le_bytes([self.read_pc(), self.read_pc()]);
				self.a = self.read(address);
				self.cycles_elapsed += 1;
			}
			/* ei */
			0xFB => {
				self.ei = true;
			}
			/* invalid opcode */
			/* invalid opcode */
			/* cp a, u8 */
			0xFE => {
				let value = self.read_pc();
				self.cp(value);
			}
			0xD3 | 0xDB | 0xDD | 0xE3..=0xE4 | 0xEB..=0xED | 0xF4 | 0xFC..=0xFD => {
				panic!("Invalid opcode")
			}
		}

		TickResult::Ok
	}
}

#[derive(Clone, Copy, Default)]
struct CarryRetainer(u8, bool);

impl CarryRetainer {
	fn new() -> Self {
		Default::default()
	}
}

impl BitOrAssign<(u8, bool)> for CarryRetainer {
	fn bitor_assign(&mut self, rhs: (u8, bool)) {
		self.0 = rhs.0;
		self.1 |= rhs.1;
	}
}

impl<S: AddressSpace> fmt::Display for State<S> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(
			f,
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
			self.a,
			self.b,
			self.c,
			self.d,
			self.e,
			self.h,
			self.l,
			self.f,
			self.pc,
			self.sp,
			if self.ei { "en" } else { "dis" },
			self.cycles_elapsed
		)
	}
}
