use clap::Parser;
use evunit::cpu;
use evunit::memory::AddressSpace;
use evunit::sym::Symfile;
use std::fs::File;
use std::io::Read;
use std::process::exit;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
	/// Path to the test configuration file
	#[clap(short, long, value_parser, value_name = "PATH")]
	config: String,

	/// Directory where crash dumps should be placed. Each dump contains the entire address space in a text file, seperated by memory type.
	#[clap(short, long, value_parser, value_name = "PATH")]
	dump_dir: Option<String>,

	/// Silence passing tests. Pass -s again to silence all output unless an error occurs.
	#[clap(short, long, action = clap::ArgAction::Count)]
	silent: u8,

	/// Path to a symfile
	#[clap(short = 'n', long, value_parser, value_name = "PATH")]
	symfile: Option<String>,

	/// Path to the ROM
	#[clap(value_parser, value_name = "PATH")]
	rom: String,
}

const SILENCE_PASSING: u8 = 1; // Silences passing messages when tests succeed.
const SILENCE_ALL: u8 = 2; // Silences all output unless an error occurs.

// All of these parameters are optional. This is because the initial values as
// well as the resulting values do not all need to be present, and in the case
// of results, may even be unknown.
struct TestConfig {
	name: String,
	a: Option<u8>,
	b: Option<u8>,
	c: Option<u8>,
	d: Option<u8>,
	e: Option<u8>,
	h: Option<u8>,
	l: Option<u8>,
	// f is decomposed into 4 bools to test them independantly.
	zf: Option<bool>,
	nf: Option<bool>,
	hf: Option<bool>,
	cf: Option<bool>,

	bc: Option<u16>,
	de: Option<u16>,
	hl: Option<u16>,
	pc: Option<u16>,
	sp: Option<u16>,

	crash_addresses: Vec<u16>,
	enable_breakpoints: bool,
	timeout: usize,

	result: Option<Box<TestConfig>>,
}

impl TestConfig {
	fn configure(&self, cpu: &mut cpu::State) {
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

	fn compare(&self, cpu: &cpu::State) -> Result<(), String> {
		let mut err_msg = String::from("");

		fn add_err<T: std::fmt::Display>(err_msg: &mut String, hint: &str, result: T, expected: T) {
			*err_msg += format!(
				"{} ({}) does not match expected value ({})\n",
				hint, result, expected
			)
			.as_str();
		}

		if let Some(value) = self.a {
			if cpu.a != value {
				add_err(&mut err_msg, "a", cpu.a, value);
			}
		}
		if let Some(value) = self.b {
			if cpu.b != value {
				add_err(&mut err_msg, "b", cpu.b, value);
			}
		}
		if let Some(value) = self.c {
			if cpu.c != value {
				add_err(&mut err_msg, "c", cpu.c, value);
			}
		}
		if let Some(value) = self.d {
			if cpu.d != value {
				add_err(&mut err_msg, "d", cpu.d, value);
			}
		}
		if let Some(value) = self.e {
			if cpu.e != value {
				add_err(&mut err_msg, "e", cpu.e, value);
			}
		}
		if let Some(value) = self.h {
			if cpu.h != value {
				add_err(&mut err_msg, "h", cpu.h, value);
			}
		}
		if let Some(value) = self.l {
			if cpu.l != value {
				add_err(&mut err_msg, "l", cpu.l, value);
			}
		}
		if let Some(value) = self.zf {
			if cpu.f.get_z() != value {
				add_err(&mut err_msg, "f.z", cpu.f.get_z(), value)
			}
		}
		if let Some(value) = self.nf {
			if cpu.f.get_n() != value {
				add_err(&mut err_msg, "f.n", cpu.f.get_n(), value)
			}
		}
		if let Some(value) = self.hf {
			if cpu.f.get_h() != value {
				add_err(&mut err_msg, "f.h", cpu.f.get_h(), value)
			}
		}
		if let Some(value) = self.cf {
			if cpu.f.get_c() != value {
				add_err(&mut err_msg, "f.c", cpu.f.get_c(), value)
			}
		}
		if let Some(value) = self.bc {
			if cpu.get_bc() != value {
				add_err(&mut err_msg, "bc", cpu.get_bc(), value)
			}
		}
		if let Some(value) = self.de {
			if cpu.get_de() != value {
				add_err(&mut err_msg, "de", cpu.get_de(), value)
			}
		}
		if let Some(value) = self.hl {
			if cpu.get_hl() != value {
				add_err(&mut err_msg, "hl", cpu.get_hl(), value)
			}
		}
		if let Some(value) = self.pc {
			if cpu.pc != value {
				add_err(&mut err_msg, "pc", cpu.pc, value)
			}
		}
		if let Some(value) = self.sp {
			if cpu.sp != value {
				add_err(&mut err_msg, "sp", cpu.sp, value)
			}
		}

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
			crash_addresses: vec![],
			enable_breakpoints: true,
			timeout: 65536,
			result: None,
		}
	}
}

fn read_config(path: &String, symfile: &Symfile) -> (TestConfig, Vec<TestConfig>) {
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

	fn parse_u16(value: &toml::Value, hint: &str, symfile: &Symfile) -> Option<u16> {
		if let toml::Value::Integer(value) = value {
			if *value < 65536 && *value >= -32768 {
				Some(*value as u16)
			} else {
				eprintln!("Value of `{hint}` must be a 16-bit integer.");
				None
			}
		} else if let toml::Value::String(value) = value {
			if let Some((_, addr)) = symfile.symbols.get(value) {
				Some(*addr)
			} else {
				eprintln!("Symbol \"{value}\" not found.");
				exit(1);
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

	fn parse_configuration(
		test: &mut TestConfig,
		key: &str,
		value: &toml::Value,
		symfile: &Symfile,
	) {
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
			"bc" => test.bc = parse_u16(value, key, symfile),
			"de" => test.de = parse_u16(value, key, symfile),
			"hl" => test.hl = parse_u16(value, key, symfile),
			"pc" => test.pc = parse_u16(value, key, symfile),
			"sp" => test.sp = parse_u16(value, key, symfile),
			"crash" => {
				if let Some(address) = parse_u16(value, key, symfile) {
					test.crash_addresses.push(address);
				}
			}
			"enable-breakpoints" => test.enable_breakpoints = parse_bool(value, key).unwrap(),
			"timeout" => {
				if let toml::Value::Integer(value) = value {
					test.timeout = *value as usize;
				} else {
					eprintln!("Value of `{key}` must be an integer.");
				}
			}
			&_ => {
				if let toml::Value::Table(value) = value {
					let mut result_config = TestConfig::new(String::from(key));
					for (key, value) in value.iter() {
						parse_configuration(&mut result_config, key, value, &symfile);
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
	let toml_file = match path.parse::<toml::Value>() {
		Ok(file) => file,
		Err(msg) => {
			eprintln!("Failed to parse config file: {msg}");
			exit(1);
		}
	};

	if let toml::Value::Table(config) = toml_file {
		for (key, value) in config.iter() {
			if let toml::Value::Table(value) = value {
				tests.push(TestConfig::new(key.to_string()));
				for (key, value) in value.iter() {
					let index = tests.len() - 1;
					parse_configuration(&mut tests[index], key, value, &symfile);
				}
			} else {
				parse_configuration(&mut global_config, key, value, &symfile);
			}
		}
	} else {
		eprintln!("TOML root is not a table (Please report this and provide the TOML file used.)");
	}

	(global_config, tests)
}

#[derive(PartialEq)]
enum FailureReason {
	None,
	Crash,
	InvalidOpcode,
	Timeout,
}

fn main() {
	fn open_input(path: &String) -> File {
		match File::open(if path == "-" { "/dev/stdin" } else { path }) {
			Ok(file) => file,
			Err(msg) => {
				eprintln!("Failed to open {path}: {msg}");
				exit(1);
			}
		}
	}

	let cli = Cli::parse();

	let rom_path = &cli.rom;
	let config_path = &cli.config;

	let address_space = match AddressSpace::open(open_input(&rom_path)) {
		Ok(result) => result,
		Err(error) => {
			eprintln!("Failed to read {rom_path}: {error}");
			exit(1);
		}
	};

	let mut config_text = String::new();
	match open_input(&config_path).read_to_string(&mut config_text) {
		Ok(..) => {}
		Err(error) => {
			eprintln!("Failed to open {config_path}: {error}");
			exit(1);
		}
	};

	let symfile = if let Some(symfile_path) = &cli.symfile {
		match Symfile::open(symfile_path) {
			Ok(result) => result,
			Err(error) => {
				eprintln!("Failed to read {symfile_path}: {error}");
				exit(1);
			}
		}
	} else {
		Symfile::new()
	};

	let (global_config, tests) = read_config(&config_text, &symfile);
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

		let failure_reason = 'tick: loop {
			match cpu_state.tick() {
				cpu::TickResult::Ok => {}
				cpu::TickResult::Halt => break FailureReason::None,
				cpu::TickResult::Stop => break FailureReason::None,
				cpu::TickResult::Break => {
					if global_config.enable_breakpoints {
						println!("{rom_path}: BREAKPOINT in {} \n{cpu_state}", test.name);
					}
				}
				cpu::TickResult::Debug => {
					if global_config.enable_breakpoints {
						println!("{rom_path}: DEBUG in {}\n{cpu_state}", test.name);
					}
				}
				cpu::TickResult::InvalidOpcode => {
					break FailureReason::InvalidOpcode;
				}
			}

			if cpu_state.pc == 0xFFFF {
				break FailureReason::None;
			}

			for addr in &global_config.crash_addresses {
				if cpu_state.pc == *addr {
					break 'tick FailureReason::Crash;
				}
			}

			for addr in &test.crash_addresses {
				if cpu_state.pc == *addr {
					break 'tick FailureReason::Crash;
				}
			}

			if cpu_state.cycles_elapsed >= global_config.timeout
				|| cpu_state.cycles_elapsed >= test.timeout
			{
				break FailureReason::Timeout;
			}
		};

		if failure_reason != FailureReason::None {
			println!(
				"\x1B[91m{}: {} failed\x1B[0m:\n{}\n{}",
				rom_path,
				test.name,
				match failure_reason {
					FailureReason::InvalidOpcode => "Invalid opcode",
					FailureReason::Crash => "Crashed",
					FailureReason::Timeout => "Timeout",
					FailureReason::None => "",
				},
				cpu_state
			);
		} else if let Some(result) = &test.result {
			match result.compare(&cpu_state) {
				Ok(..) => {
					if cli.silent < SILENCE_PASSING {
						println!("\x1B[92m{}: {} passed\x1B[0m", rom_path, test.name);
					}
					continue;
				}
				Err(msg) => {
					print!(
						"\x1B[91m{}: {} failed\x1B[0m:\n{}",
						rom_path, test.name, msg
					);
				}
			}
		} else {
			if cli.silent < SILENCE_PASSING {
				println!("\x1B[92m{}: {} passed\x1B[0m", rom_path, test.name);
			}
			continue; // This continue skips failure handling.
		}

		if let Some(ref dump_dir) = cli.dump_dir {
			let path = String::from(dump_dir) + format!("/{}.txt", test.name).as_str();

			match File::create(&path) {
				Ok(file) => match cpu_state.address_space.dump(file) {
					Ok(..) => {}
					Err(msg) => eprintln!("Failed to write dump to {path}: {msg}"),
				},
				Err(msg) => eprintln!("Failed to open {path}: {msg}"),
			}
		}
		fail_count += 1;
	}

	// When in SILENCE_ALL only print the final message if a test failed.
	if !(cli.silent >= SILENCE_ALL) || fail_count != 0 {
		println!(
			"{}: All tests complete. {}/{} passed.",
			rom_path,
			tests.len() - fail_count,
			tests.len()
		);
	}

	if fail_count > 0 {
		exit(1);
	}
}
