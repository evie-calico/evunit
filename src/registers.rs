use crate::{Error, Result};
use gb_cpu_sim::{cpu, memory};
use paste::paste;
use std::fmt;
use owo_colors::OwoColorize;

#[derive(Clone, Debug)]
enum CompareSource {
	Register(&'static str),
	Address(u16),
}

impl fmt::Display for CompareSource {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			CompareSource::Register(name) => write!(f, "{name}"),
			CompareSource::Address(address) => write!(f, "[{address:X}]"),
		}
	}
}

#[derive(Clone, Debug, Default)]
pub struct CompareResult {
	contents: Vec<(CompareSource, String, String)>,
}

impl fmt::Display for CompareResult {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		for (source, result, expected) in &self.contents {
			writeln!(
				f,
				"{source} ({result}) does not match expected value ({expected})",
				source = source.bold().bright_white(),
				result = result.cyan(),
				expected = expected.cyan()
			)?;
		}
		Ok(())
	}
}

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
			#[must_use]
			pub fn [<with_ $reg>](mut self, value: $type) -> Self {
				self.$reg = Some(value);
				self
			}
		}
	};
}

impl Default for Registers {
	fn default() -> Self {
		Self::new()
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

	/// Compares this set of registers to the CPU, returning an error if they do not match.
	///
	/// # Errors
	///
	/// Returns an error if the CPU's state does not match `self`
	/// The error message contains a list of the values that did not match.
	pub fn compare<S: memory::AddressSpace>(&self, cpu: &cpu::State<S>) -> Result<()> {
		let mut errors = CompareResult::default();

		macro_rules! check {
			(impl $cfg:ident, $name:expr, $cpu:expr) => {
				if let Some(value) = self.$cfg {
					if $cpu != value {
						errors.contents.push((
							CompareSource::Register(stringify!($name)),
							$cpu.to_string(),
							value.to_string()
						))
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
				errors.contents.push((
					CompareSource::Address(*addr),
					result.to_string(),
					value.to_string(),
				));
			}
		}

		if errors.contents.is_empty() {
			Ok(())
		} else {
			Err(Error::CompareFailed(errors))
		}
	}

	#[must_use]
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
