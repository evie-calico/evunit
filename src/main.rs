use rgbunit::cpu;
use rgbunit::memory;
use std::env;
use std::process::exit;

// All of these parameters are optional. This is because the initial values as
// well as the resulting values do not all need to be present, and in the case
// of results, may even be unknown.
#[derive(Debug)]
struct TestConfig {
	name: String,
	a: Option<u8>,
	b: Option<u8>, c: Option<u8>,
	d: Option<u8>, e: Option<u8>,
	h: Option<u8>, l: Option<u8>,
	// f is decomposed into 4 bools to test them independantly.
	zf: Option<bool>,
	nf: Option<bool>,
	hf: Option<bool>,
	cf: Option<bool>,

	pc: Option<u16>,
	sp: Option<u16>,

	result: Option<Box<TestConfig>>,
}

impl TestConfig {
	fn configure(&self, cpu: &mut cpu::State) {
		// Macros should be able to do something like this?
		if let Some(value) = self.a { cpu.a = value }
		if let Some(value) = self.b { cpu.b = value }
		if let Some(value) = self.c { cpu.c = value }
		if let Some(value) = self.d { cpu.d = value }
		if let Some(value) = self.e { cpu.e = value }
		if let Some(value) = self.h { cpu.h = value }
		if let Some(value) = self.l { cpu.l = value }
		if let Some(value) = self.zf { cpu.f.set_z(value) }
		if let Some(value) = self.nf { cpu.f.set_n(value) }
		if let Some(value) = self.hf { cpu.f.set_h(value) }
		if let Some(value) = self.cf { cpu.f.set_c(value) }
		if let Some(value) = self.pc { cpu.pc = value }
		if let Some(value) = self.sp { cpu.sp = value }
	}

	fn compare(&self, cpu: &cpu::State) -> Result<(), String> {
		let mut err_msg = String::from("");

		fn add_err<T: std::fmt::Display>(err_msg: &mut String, hint: &str, result: T, expected: T) {
			*err_msg += format!("{} ({}) does not match expected value ({})\n", hint, result, expected).as_str();
		}

		if let Some(value) = self.a { if cpu.a != value { add_err(&mut err_msg, "a", cpu.a, value); } }
		if let Some(value) = self.b { if cpu.b != value { add_err(&mut err_msg, "b", cpu.b, value); } }
		if let Some(value) = self.c { if cpu.c != value { add_err(&mut err_msg, "c", cpu.c, value); } }
		if let Some(value) = self.d { if cpu.d != value { add_err(&mut err_msg, "d", cpu.d, value); } }
		if let Some(value) = self.e { if cpu.e != value { add_err(&mut err_msg, "e", cpu.e, value); } }
		if let Some(value) = self.h { if cpu.h != value { add_err(&mut err_msg, "h", cpu.h, value); } }
		if let Some(value) = self.l { if cpu.l != value { add_err(&mut err_msg, "l", cpu.l, value); } }
		if let Some(value) = self.zf { if cpu.f.get_z() != value { add_err(&mut err_msg, "f.z", cpu.f.get_z(), value) } }
		if let Some(value) = self.nf { if cpu.f.get_n() != value { add_err(&mut err_msg, "f.n", cpu.f.get_n(), value) } }
		if let Some(value) = self.hf { if cpu.f.get_h() != value { add_err(&mut err_msg, "f.h", cpu.f.get_h(), value) } }
		if let Some(value) = self.cf { if cpu.f.get_c() != value { add_err(&mut err_msg, "f.c", cpu.f.get_c(), value) } }
		if let Some(value) = self.pc { if cpu.pc != value { add_err(&mut err_msg, "pc", cpu.pc, value) } }
		if let Some(value) = self.sp { if cpu.sp != value { add_err(&mut err_msg, "sp", cpu.sp, value) } }

		if err_msg.len() == 0 {
			Ok(())
		} else {
			Err(err_msg)
		}
	}

	fn new(name: String) -> TestConfig {
		TestConfig {
			name,
			a: None,
			b: None, c: None,
			d: None, e: None,
			h: None, l: None,
			zf: None, nf: None, hf: None, cf: None,
			pc: None,
			sp: None,
			result: None,
		}
	}
}

fn read_config(path: &String) -> (TestConfig, Vec<TestConfig>) {
	fn parse_u8(value: &toml::Value, hint: &str) -> Option<u8> {
		if let toml::Value::Integer(value) = value {
			if *value < 256 && *value >= -128 {
				Some(*value as u8)
			} else {
				eprintln!("Value of `{hint}` must be an 8-bit integer.");
				None
			}
		} else {
			eprintln!("Value of `{hint}` must be an 8-bit integer.");
			None
		}
	}

	fn parse_u16(value: &toml::Value, hint: &str) -> Option<u16> {
		if let toml::Value::Integer(value) = value {
			if *value < 65536 && *value >= -32768 {
				Some(*value as u16)
			} else {
				eprintln!("Value of `{hint}` must be a 16-bit integer.");
				None
			}
		} else {
			eprintln!("Value of `{hint}` must be a 16-bit integer.");
			None
		}
	}

	fn parse_bool(value: &toml::Value, hint: &str) -> Option<bool> {
		if let toml::Value::Boolean(value) = value {
			Some(*value)
		} else {
			eprintln!("Value of `{hint}` must be an 8-bit integer.");
			None
		}
	}

	fn parse_configuration(test: &mut TestConfig, key: &str, value: &toml::Value) {
		match key {
			"a" => test.a = parse_u8(value, key),
			"b" => test.b = parse_u8(value, key),
			"c" => test.c = parse_u8(value, key),
			"d" => test.d = parse_u8(value, key),
			"e" => test.e = parse_u8(value, key),
			"h" => test.h = parse_u8(value, key),
			"l" => test.l = parse_u8(value, key),
			"f.z" => test.zf = parse_bool(value, key),
			"f.n" => test.nf = parse_bool(value, key),
			"f.h" => test.hf = parse_bool(value, key),
			"f.c" => test.cf = parse_bool(value, key),
			"pc" => test.pc = parse_u16(value, key),
			"sp" => test.sp = parse_u16(value, key),
			&_ => {
				if let toml::Value::Table(value) = value {
					let mut result_config = TestConfig::new(String::from(key));
					for (key, value) in value.iter() {
						parse_configuration(&mut result_config, key, value);
					}
					test.result = Some(Box::new(result_config));
				} else {
					println!("Unknown config {key} = {value:?}");					
				}
			}
		}
	}

	let mut global_config = TestConfig::new(String::from("Global"));
	let mut tests: Vec<TestConfig> = vec![];
	let toml_file = path.parse::<toml::Value>().unwrap();

	if let toml::Value::Table(config) = toml_file {
		for (key, value) in config.iter() {
			if let toml::Value::Table(value) = value {
				tests.push(TestConfig::new(key.to_string()));
				for (key, value) in value.iter() {
					let index = tests.len() - 1;
					parse_configuration(&mut tests[index], key, value);
				}
			} else {
				parse_configuration(&mut global_config, key, value);
			}
		}
	}

	(global_config, tests)
}

fn main() {
	let args: Vec<String> = env::args().collect();
	if args.len() < 2 {
		eprintln!("usage: {} <rom path>", args[0]);
		exit(1);
	}

	let rom_path = &args[1];

	let address_space = match memory::AddressSpace::open(rom_path) {
		Ok(result) => result,
		Err(error) => {
			eprintln!("Failed to open {}: {}", rom_path, error);
			exit(1);
		}
	};

	let (global_config, tests) = read_config(&String::from("a = 0\nsp = 0xD000\n[add]\nb = 1\n[add.result]\na = 1"));
	let mut fail_count = 0;

	for test in &tests {
		let mut cpu_state = cpu::State::new(address_space.clone());
		global_config.configure(&mut cpu_state);
		test.configure(&mut cpu_state);

		// Push the return address 0xFFFF onto the stack.
		// If pc == 0xFFFF the test is complete.
		// TODO: make the success address configurable.
		cpu_state.write(cpu_state.sp - 1, 0xFF);
		cpu_state.write(cpu_state.sp - 2, 0xFF);
		cpu_state.sp -= 2;

		loop {
			let mut test_complete = false;

			match cpu_state.tick() {
				cpu::TickResult::Ok() => {},
				cpu::TickResult::Stop() => test_complete = true,
				cpu::TickResult::Break() => { println!("{cpu_state:#?}\n"); },
				cpu::TickResult::Debug() => { println!("{cpu_state:#?}\n"); },
			}

			test_complete = test_complete || cpu_state.pc == 0xFFFF;

			if test_complete { break }
		}

		if let Some(result) = &test.result {
			match result.compare(&cpu_state) {
				Ok(..) => println!("\x1B[92m{}: {} passed\x1B[0m", rom_path, test.name),
				Err(msg) => {
					println!("\x1B[91m{}: {} failed\x1B[0m:\n{}", rom_path, test.name, msg);
					fail_count += 1;
				}
			}
		} else {
			println!("{} {} exited", rom_path, test.name);
		}
	}

	println!("{}: All tests complete. {}/{} passed.", rom_path, tests.len() - fail_count, tests.len());
	if fail_count > 0 { exit(1); }
}
