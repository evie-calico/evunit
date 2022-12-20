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

	pub bc: Option<u16>,
	pub de: Option<u16>,
	pub hl: Option<u16>,
	pub pc: Option<u16>,
	pub sp: Option<u16>,
}

impl Registers {
	pub fn configure<S: memory::AddressSpace>(&self, cpu: &mut cpu::State<S>) {
		// Macros should be able to do something like this?
		if let Some(value) = self.a {
			cpu.a = value
		}
		if let Some(value) = self.b {
			cpu.b = value
		}
		if let Some(value) = self.c {
			cpu.c = value
		}
		if let Some(value) = self.d {
			cpu.d = value
		}
		if let Some(value) = self.e {
			cpu.e = value
		}
		if let Some(value) = self.h {
			cpu.h = value
		}
		if let Some(value) = self.l {
			cpu.l = value
		}
		if let Some(value) = self.zf {
			cpu.f.set_z(value)
		}
		if let Some(value) = self.nf {
			cpu.f.set_n(value)
		}
		if let Some(value) = self.hf {
			cpu.f.set_h(value)
		}
		if let Some(value) = self.cf {
			cpu.f.set_c(value)
		}
		if let Some(value) = self.bc {
			cpu.set_bc(value)
		}
		if let Some(value) = self.de {
			cpu.set_de(value)
		}
		if let Some(value) = self.hl {
			cpu.set_hl(value)
		}
		if let Some(value) = self.pc {
			cpu.pc = value
		}
		if let Some(value) = self.sp {
			cpu.sp = value
		}
	}

	pub fn compare<S: memory::AddressSpace>(&self, cpu: &cpu::State<S>) -> Result<(), String> {
		let mut err_msg = String::from("");

		fn add_err<T: std::fmt::Display>(err_msg: &mut String, hint: &str, result: T, expected: T) {
			*err_msg += format!(
				"{} ({}) does not match expected value ({})\n",
				hint, result, expected
			)
			.as_str();
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

		if err_msg.is_empty() {
			Ok(())
		} else {
			Err(err_msg)
		}
	}

	pub fn new() -> Registers {
		Registers {
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
		}
	}
}
