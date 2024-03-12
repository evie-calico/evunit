use clap::Parser;
use evunit::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{stdin, BufReader, Read};
use std::process::exit;

pub const SILENCE_NONE: u8 = 0;
pub const SILENCE_PASSING: u8 = 1; // Silences passing messages when tests succeed.
pub const SILENCE_ALL: u8 = 2; // Silences all output unless an error occurs.

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

fn read_config(path: &str, symfile: &HashMap<String, (u32, u16)>) -> Vec<TestConfig> {
	fn parse_u8(value: &toml::Value, hint: &str) -> Option<u8> {
		match value {
			toml::Value::Integer(value) => {
				if -128 <= *value && *value < 256 {
					Some(*value as u8)
				} else {
					eprintln!("Value of `{hint}` must be an 8-bit integer.");
					None
				}
			}
			_ => {
				eprintln!("Value of `{hint}` must be an 8-bit integer.");
				None
			}
		}
	}

	fn parse_u16(
		value: &toml::Value,
		hint: &str,
		symfile: &HashMap<String, (u32, u16)>,
	) -> Option<u16> {
		match value {
			toml::Value::Integer(value) => {
				if -32768 <= *value && *value < 65536 {
					Some(*value as u16)
				} else {
					eprintln!("Value of `{hint}` must be a 16-bit integer.");
					None
				}
			}
			toml::Value::String(value) => {
				if let Some((_, addr)) = symfile.get(value) {
					Some(*addr)
				} else {
					eprintln!("Symbol \"{value}\" not found.");
					exit(1);
				}
			}
			_ => {
				eprintln!("Value of `{hint}` must be a 16-bit integer.");
				None
			}
		}
	}

	fn parse_bool(value: &toml::Value, hint: &str) -> Option<bool> {
		if let toml::Value::Boolean(value) = value {
			Some(*value)
		} else {
			eprintln!("Value of `{hint}` must be a boolean.");
			None
		}
	}

	fn parse_address(address: &str, symfile: &HashMap<String, (u32, u16)>) -> Option<u16> {
		if let Some((_, address)) = symfile.get(address) {
			// Attempt to get address from symfile
			Some(*address)
		} else if let Ok(address) = u16::deserialize(toml::de::ValueDeserializer::new(address)) {
			// Attempt to parse address as an u16 value
			Some(address)
		} else {
			// Failed to parse address
			None
		}
	}

	fn parse_memory(name: &str, value: &toml::Value) -> Result<Vec<u8>, String> {
		match value {
			toml::Value::Integer(value) => {
				if *value > 255 || *value < -128 {
					let value_array = (*value as i64)
						.to_le_bytes()
						.iter()
						.skip_while(|b| **b == 0)
						.map(|b| format!("0x{b:02x}"))
						.collect::<Vec<_>>()
						.join(", ");

					// Disallow non 8-bit values and present alternative
					Err(format!(
						"\"{value}\" is not an 8-bit value. Try \"[{value_array}]\" instead."
					))
				} else {
					// Treat any byte size number as a byte
					Ok(vec![(*value as u8)])
				}
			}
			toml::Value::String(value) => {
				if !value.is_ascii() {
					// Disallow any strings which contain non-ASCII values
					Err(format!(
						"String value \"{value}\" contains non-ASCII characters"
					))
				} else {
					// Convert string into sequence of bytes
					Ok(value.bytes().collect::<Vec<_>>())
				}
			}
			toml::Value::Array(value) => {
				// Recursively call function on all toml::Value and return their collected result
				value
					.iter()
					.map(|v| parse_memory(name, v))
					.collect::<Result<Vec<Vec<u8>>, String>>()
					.map(|mem| mem.into_iter().flatten().collect::<Vec<u8>>())
			}
			toml::Value::Boolean(value) => {
				// Convert bool into either a 1 or a 0
				Ok(vec![if *value { 1 } else { 0 }])
			}
			_ => {
				// Other types return error as they are not supported
				Err(format!("Unsupported value for {name}: {value}"))
			}
		}
	}

	fn parse_memory_assignment(
		name: &str,
		value: &toml::Value,
		symfile: &HashMap<String, (u32, u16)>,
	) -> Result<Vec<(u16, u8)>, String> {
		let address = parse_address(name, symfile);
		if address.is_none() {
			return Err(format!("Address \"{}\" is not a valid address", name));
		}
		let address = address.unwrap();

		parse_memory(name, value).map(|data| {
			data.iter()
				.enumerate()
				.map(|(i, b)| (address + (i as u16), *b))
				.collect::<Vec<(u16, u8)>>()
		})
	}

	fn parse_configuration(
		test: &mut TestConfig,
		key: &str,
		value: &toml::Value,
		symfile: &HashMap<String, (u32, u16)>,
	) {
		match key {
			"a" => test.initial.a = parse_u8(value, key),
			"b" => test.initial.b = parse_u8(value, key),
			"c" => test.initial.c = parse_u8(value, key),
			"d" => test.initial.d = parse_u8(value, key),
			"e" => test.initial.e = parse_u8(value, key),
			"h" => test.initial.h = parse_u8(value, key),
			"l" => test.initial.l = parse_u8(value, key),
			"f.z" => test.initial.zf = parse_bool(value, key),
			"f.n" => test.initial.nf = parse_bool(value, key),
			"f.h" => test.initial.hf = parse_bool(value, key),
			"f.c" => test.initial.cf = parse_bool(value, key),
			"bc" => test.initial.bc = parse_u16(value, key, symfile),
			"de" => test.initial.de = parse_u16(value, key, symfile),
			"hl" => test.initial.hl = parse_u16(value, key, symfile),
			"pc" => test.initial.pc = parse_u16(value, key, symfile),
			"sp" => test.initial.sp = parse_u16(value, key, symfile),
			"caller" => test.caller_address = parse_u16(value, key, symfile).unwrap_or(0xFFFF),
			"crash" => {
				if let toml::Value::Integer(_) | toml::Value::String(_) = value {
					if let Some(address) = parse_u16(value, key, symfile) {
						test.crash_addresses.push(address);
					}
				} else if let toml::Value::Array(addresses) = value {
					for i in addresses {
						if let Some(address) = parse_u16(i, key, symfile) {
							test.crash_addresses.push(address);
						}
					}
				} else {
					eprintln!(
						"Value of {key} must be a 16-bit integer or an array of 16-bit integers"
					)
				}
			}
			"enable-breakpoints" => test.enable_breakpoints = parse_bool(value, key).unwrap(),
			"exit" => {
				if let toml::Value::Integer(_) | toml::Value::String(_) = value {
					if let Some(address) = parse_u16(value, key, symfile) {
						test.exit_addresses.push(address);
					}
				} else if let toml::Value::Array(addresses) = value {
					for i in addresses {
						if let Some(address) = parse_u16(i, key, symfile) {
							test.exit_addresses.push(address);
						}
					}
				} else {
					eprintln!(
						"Value of {key} must be a 16-bit integer or an array of 16-bit integers"
					)
				}
			}
			"timeout" => {
				if let toml::Value::Integer(value) = value {
					test.timeout = *value as usize;
				} else {
					eprintln!("Value of `{key}` must be an integer.");
				}
			}
			"result" => {
				if let toml::Value::Table(value) = value {
					let mut result = Registers::new();
					for (key, value) in value {
						match key.as_str() {
							"a" => result.a = parse_u8(value, key),
							"b" => result.b = parse_u8(value, key),
							"c" => result.c = parse_u8(value, key),
							"d" => result.d = parse_u8(value, key),
							"e" => result.e = parse_u8(value, key),
							"h" => result.h = parse_u8(value, key),
							"l" => result.l = parse_u8(value, key),
							"f.z" => result.zf = parse_bool(value, key),
							"f.n" => result.nf = parse_bool(value, key),
							"f.h" => result.hf = parse_bool(value, key),
							"f.c" => result.cf = parse_bool(value, key),
							"bc" => result.bc = parse_u16(value, key, symfile),
							"de" => result.de = parse_u16(value, key, symfile),
							"hl" => result.hl = parse_u16(value, key, symfile),
							"pc" => result.pc = parse_u16(value, key, symfile),
							"sp" => result.sp = parse_u16(value, key, symfile),
							&_ => {
								let mut indices = key.char_indices();
								if let (Some((_, '[')), Some((begin, _)), Some((end, ']'))) =
									(indices.next(), indices.next(), indices.last())
								{
									match parse_memory_assignment(&key[begin..end], value, symfile) {
										Err(cause) => eprintln!("{}", cause),
										Ok(data) => result.memory = data,
									};
								} else {
									eprintln!("Unknown config key {key} = {value:?}");
								}
							}
						}
					}
					test.result = Some(result);
				} else {
					eprintln!("Value of `{key}` must be a table.");
				}
			}
			"stack" => {
				match parse_memory("stack", value) {
					Err(cause) => eprintln!("{}", cause),
					Ok(data) => test.stack.extend(data),
				};
			}
			_ => {
				let mut indices = key.char_indices();
				if let (Some((_, '[')), Some((begin, _)), Some((end, ']'))) =
					(indices.next(), indices.next(), indices.last())
				{
					match parse_memory_assignment(&key[begin..end], value, symfile) {
						Err(cause) => eprintln!("{}", cause),
						Ok(data) => test.initial.memory = data,
					};
				} else {
					eprintln!("Unknown config key {key} = {value:?}");
				}
			}
		}
	}

	let mut global_config = TestConfig::new(String::from("Global"));
	let mut tests: Vec<TestConfig> = vec![];
	let toml_file = path.parse::<toml::Value>().unwrap_or_else(|msg| {
		eprintln!("Failed to parse config file: {msg}");
		exit(1);
	});

	let config = if let toml::Value::Table(config) = toml_file {
		config
	} else {
		panic!("TOML root is not a table (Please report this and provide the TOML file used.)");
	};

	for (key, value) in config {
		if let toml::Value::Table(table) = value {
			let mut test = global_config.clone();
			test.name = key;
			for (key, value) in table.iter() {
				parse_configuration(&mut test, key, value, symfile);
			}
			tests.push(test);
		} else {
			parse_configuration(&mut global_config, &key, &value, symfile);
		}
	}

	tests
}

fn main() {
	fn open_input(path: &str) -> Box<dyn Read> {
		if path == "-" {
			Box::new(BufReader::new(stdin()))
		} else {
			Box::new(File::open(path).unwrap_or_else(|msg| {
				eprintln!("Failed to open {path}: {msg}");
				exit(1)
			}))
		}
	}

	let cli = Cli::parse();

	let rom_path = cli.rom;
	let config_path = cli.config;

	let rom = open_rom(&rom_path);

	let address_space = AddressSpace::with(&rom);

	let mut config_text = String::new();
	open_input(&config_path)
		.read_to_string(&mut config_text)
		.unwrap_or_else(|error| {
			eprintln!("Failed to read {config_path}: {error}");
			exit(1);
		});

	let symfile = open_symfile(cli.symfile.as_ref().map(|x| x.as_ref()));
	let tests = read_config(&config_text, &symfile);

	let silence_level = match cli.silent {
		SILENCE_NONE => SilenceLevel::None,
		SILENCE_PASSING => SilenceLevel::Passing,
		SILENCE_ALL.. => SilenceLevel::All,
	};

	let mut logger = Logger::new(silence_level, &rom_path);

	// create dump dir if it does not exist already
	if let Some(ref dump_dir) = cli.dump_dir {
		if let Err(msg) = fs::create_dir_all(dump_dir) {
			eprintln!("Failed to create dump dir {dump_dir}: {msg}");
			exit(1);
		}
	}

	for test in &tests {
		let mut cpu_state = cpu::State::new(address_space.clone());
		let mut test_logger = logger.make_test(test);

		if test.run(&mut cpu_state, &mut test_logger) {
			continue;
		}

		if let Some(ref dump_dir) = cli.dump_dir {
			let path = String::from(dump_dir) + &format!("/{}.txt", test.name);

			match File::create(&path) {
				Ok(file) => cpu_state.address_space.dump(file).unwrap_or_else(|msg| {
					eprintln!("Failed to write dump to {path}: {msg}");
				}),
				Err(msg) => eprintln!("Failed to open {path}: {msg}"),
			}
		}
	}

	if !logger.finish() {
		exit(1);
	}
}
