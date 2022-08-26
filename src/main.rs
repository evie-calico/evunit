use rgbunit::cpu;
use rgbunit::memory;
use std::env;
use std::process::exit;

#[derive(Debug)]
struct TestConfig {
	name: String,
	a: u8, f: cpu::Flags,
	b: u8, c: u8,
	d: u8, e: u8,
	h: u8, l: u8,
	pc: u16,
	sp: u16,

	result: Option<Box<TestConfig>>,
}

impl TestConfig {
	fn configure(&self, cpu: &mut cpu::State) {
		cpu.a = self.a;
		cpu.f.value = self.f.value;
		cpu.b = self.b;
		cpu.c = self.c;
		cpu.d = self.d;
		cpu.e = self.e;
		cpu.h = self.h;
		cpu.l = self.l;
		cpu.pc = self.pc;
		cpu.sp = self.sp;
	}

	fn new(name: String) -> TestConfig {
		TestConfig {
			name,
			a: 0, f: cpu::Flags { value: 0 },
			b: 0, c: 0,
			d: 0, e: 0,
			h: 0, l: 0,
			pc: 0,
			sp: 0xE000,
			result: None,
		}
	}
}

fn read_config(path: &String) -> (TestConfig, Vec<TestConfig>) {
	fn parse_u8(value: &toml::Value, hint: &str) -> u8 {
		if let toml::Value::Integer(value) = value {
			if *value < 256 && *value >= -128 {
				*value as u8
			} else {
				eprintln!("Value of `{hint}` must be an 8-bit integer.");
				0
			}
		} else {
			eprintln!("Value of `{hint}` must be an 8-bit integer.");
			0
		}
	}

	fn parse_u16(value: &toml::Value, hint: &str) -> u16 {
		if let toml::Value::Integer(value) = value {
			if *value < 65536 && *value >= -32768 {
				*value as u16
			} else {
				eprintln!("Value of `{hint}` must be a 16-bit integer.");
				0
			}
		} else {
			eprintln!("Value of `{hint}` must be a 16-bit integer.");
			0
		}
	}

	fn parse_bool(value: &toml::Value, hint: &str) -> bool {
		if let toml::Value::Boolean(value) = value {
			*value
		} else {
			eprintln!("Value of `{hint}` must be an 8-bit integer.");
			false
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
			"f" => test.f.value = parse_u8(value, key),
			"f.z" => test.f.set_z(parse_bool(value, key)),
			"f.n" => test.f.set_n(parse_bool(value, key)),
			"f.h" => test.f.set_h(parse_bool(value, key)),
			"f.c" => test.f.set_c(parse_bool(value, key)),
			"pc" => test.pc = parse_u16(value, key),
			"sp" => test.sp = parse_u16(value, key),
			&_ => {
				if let toml::Value::Table(value) = value {
					for (key, value) in value.iter() {
						let mut result_config = TestConfig::new(String::from("Result"));
						parse_configuration(&mut result_config, key, value);
						test.result = Some(Box::new(result_config));
					}
				} else {
					println!("Unknown config {key} = {value:?}");					
				}
			}
		}
	}

	let mut global_config = TestConfig::new(String::from("Global"));
	let mut tests: Vec<TestConfig> = vec![];

	if let toml::Value::Table(config) = path.parse::<toml::Value>().unwrap() {
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

	for test in tests {
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

		println!("{} {} passed", rom_path, test.name);
		println!("{cpu_state:#?}");
	}
}
