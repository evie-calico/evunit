use gb_cpu_sim::{cpu, memory};

use paste::paste;

// All of these parameters are optional. This is because the initial values as
// well as the resulting values do not all need to be present, and in the case
// of results, may even be unknown.
#[derive(Debug, Clone)]
pub struct Registers {
	pub a: Option<u8>,
	pub b: Option<u8>,
	pub c: Option<u8>,
	pub d: Option<u8>,
	pub e: Option<u8>,
	pub h: Option<u8>,
	pub l: Option<u8>,
	// f is decomposed into 4 bools to test them independantly.
	pub zf: Option<bool>,
	pub nf: Option<bool>,
	pub hf: Option<bool>,
	pub cf: Option<bool>,
	// TODO: These 16-bit registers make sense in the config file, but should they be part of this struct?
	pub bc: Option<u16>,
	pub de: Option<u16>,
	pub hl: Option<u16>,

	pub pc: Option<u16>,
	pub sp: Option<u16>,

	// Each byte in memory may have a value.
	// For very very large configs this may have a higher memory usage.
	// If this becomes a problem, consider moving AddressSpace here.
	pub memory: Vec<(u16, u8)>,
}

macro_rules! impl_with {
	($reg:ident : $type:ty) => {
		paste! {
			pub fn [<with_ $reg>](mut self, value: $type) -> Self {
				self.$reg = Some(value);
				self
			}
		}
	}
}

impl Registers {
	pub fn configure<S: memory::AddressSpace>(&self, cpu: &mut cpu::State<S>) {
		macro_rules! optional_set {
			($cfg:ident) => {
				if let Some(value) = self.$cfg {
					cpu.$cfg = value;
				}
			};
			(f $cfg:ident) => {
				paste! {
					if let Some(value) = self.[<$cfg f>] {
						 cpu.f.[<set_ $cfg>](value);
					}
				}
			};
			(set $cfg:ident) => {
				if let Some(value) = self.$cfg {
					paste! { cpu.[<set_ $cfg>](value); }
				}
			};
			($($($i:ident)+),+) => { $( optional_set!($($i)+); )+ };
		}

		optional_set!(a, b, c, d, e, h, l);
		optional_set!(f z, f n, f h, f c);
		optional_set!(set bc, set de, set hl, pc, sp);

		for (addr, value) in &self.memory {
			cpu.address_space.write(*addr, *value);
		}
	}

	pub fn compare<S: memory::AddressSpace>(&self, cpu: &cpu::State<S>) -> Result<(), String> {
		let mut err_msg = String::from("");

		fn add_err<T: std::fmt::Display>(err_msg: &mut String, hint: &str, result: T, expected: T) {
			*err_msg += &format!("{hint} ({result}) does not match expected value ({expected})\n");
		}

		macro_rules! check {
			(impl $cfg:ident, $name:expr, $cpu:expr) => {
				if let Some(value) = self.$cfg {
					if $cpu != value {
						add_err(&mut err_msg, stringify!($name), $cpu, value);
					}
				}
			};
			($reg:ident) => {
				check!(impl $reg, $reg, cpu.$reg)
			};
			(get $reg:ident) => {
				paste! { check!(impl $reg, $reg, cpu.[<get_ $reg>]()) }
			};
			(f $flag:ident) => {
				paste! { check!(impl [<$flag f>], f.$flag, cpu.f.[<get_ $flag>]()) }
			};
			($($($i:ident)+),+) => { $( check!($($i)+); )+ };
		}
		check!(a, b, c, d, e, h, l);
		check!(f z, f n, f h, f c);
		check!(get bc, get de, get hl, sp, pc);

		for (addr, value) in &self.memory {
			let result = cpu.address_space.read(*addr);
			if result != *value {
				err_msg += &format!("[${addr:X}] ({result}) does not match expected value ({value})\n");
			}
		}

		if err_msg.is_empty() {
			Ok(())
		} else {
			Err(err_msg)
		}
	}

	pub fn new() -> Self {
		Self {
			a: None,
			b: None,
			c: None,
			d: None,
			e: None,
			h: None,
			l: None,
			zf: None,
			nf: None,
			hf: None,
			cf: None,
			bc: None,
			de: None,
			hl: None,
			pc: None,
			sp: None,
			memory: Vec::new(),
		}
	}

	impl_with!(a: u8);
	impl_with!(b: u8);
	impl_with!(c: u8);
	impl_with!(d: u8);
	impl_with!(e: u8);
	impl_with!(h: u8);
	impl_with!(l: u8);
	impl_with!(zf: bool);
	impl_with!(nf: bool);
	impl_with!(hf: bool);
	impl_with!(cf: bool);
	impl_with!(bc: u16);
	impl_with!(de: u16);
	impl_with!(hl: u16);
	impl_with!(pc: u16);
	impl_with!(sp: u16);
}
