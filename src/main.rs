mod log;
mod memory;
mod registers;
mod test;

use clap::Parser;
use gb_cpu_sim::cpu;

use std::collections::HashMap;
use std::fs::File;
use std::io::{stdin, BufRead, BufReader, Read};
use std::process::exit;

use crate::log::Logger;
use crate::memory::AddressSpace;
use crate::registers::Registers;
use crate::test::TestConfig;

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

	fn parse_memory(
		value: &toml::Value,
		symbol: &str, 
		symfile: &HashMap<String, (u32, u16)>,
		memory: &mut Vec<(u16, u8)>
	) {
		let addr = if let Some((_, addr)) = symfile.get(symbol) {
			*addr
		} else {
			eprintln!("Symbol \"{symbol}\" not found.");
			return;
		};

		parse_memory_at_addr(addr, value, symbol, symfile, memory);

		fn parse_memory_at_addr(
			mut addr: u16,
			value: &toml::Value,
			symbol: &str, 
			symfile: &HashMap<String, (u32, u16)>,
			memory: &mut Vec<(u16, u8)>
		) -> u16 {
			match value {
				toml::Value::Integer(value) => {
					if *value > 255 || *value < -128 {
						let mut value_array = String::new();
						let mut working_value = *value as u32;
						loop {
							// We can do at least one loop because the value has over 8 bits.
							value_array += &(working_value & 0xFF).to_string();
							working_value >>= 8;
							if working_value == 0 { break; }
							value_array += ", ";
						}
						eprintln!("{symbol}'s value ({value}) is not 8-bit. Try `\"{symbol}\" = [{value_array}]` instead.");
					} else {
						memory.push((addr, *value as u8));
					}
					addr + 1
				}
				toml::Value::String(value) => {
					for i in value.bytes() {
						memory.push((addr, i));
						addr += 1;
					}
					addr
				}
				toml::Value::Array(value) => {
					for i in value {
						addr = parse_memory_at_addr(addr, i, symbol, symfile, memory);
					}
					addr
				}
				toml::Value::Boolean(value) => {
					memory.push((addr, *value as u8));
					addr + 1
				}
				_ => {
					eprintln!("Unsupported value for {symbol}: {value}");
					addr
				}
			}
		}
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
					eprintln!("Value of {key} must be a 16-bit integer or an array of 16-bit integers")
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
					eprintln!("Value of {key} must be a 16-bit integer or an array of 16-bit integers")
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
								if let (Some((_, '[')), Some((begin, _)), Some((end, ']')))
								= (indices.next(), indices.next(), indices.last()) {
									parse_memory(value, &key[begin..end].to_string(), symfile, &mut result.memory);
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
			_ => {
				let mut indices = key.char_indices();
				if let (Some((_, '[')), Some((begin, _)), Some((end, ']')))
				= (indices.next(), indices.next(), indices.last()) {
					parse_memory(value, &key[begin..end].to_string(), symfile, &mut test.initial.memory);
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

fn read_symfile(path: &Option<String>) -> HashMap<String, (u32, u16)> {
	let mut symfile = HashMap::new();
	if let Some(symfile_path) = &path {
		let file = File::open(symfile_path).unwrap_or_else(|error| {
			eprintln!("Failed to open {symfile_path}: {error}");
			exit(1);
		});
		let symbols = BufReader::new(file)
			.lines()
			.map(|line| {
				line.unwrap_or_else(|error| {
					eprintln!("Error reading {symfile_path}: {error}");
					exit(1);
				})
			})
			.enumerate()
			.filter_map(|(n, line)| {
				gb_sym_file::parse_line(&line).map(|parse_result| {
					parse_result.unwrap_or_else(|parse_error| {
						eprintln!(
							"Failed to parse {symfile_path} line {}: {parse_error}",
							n + 1
						);
						exit(1);
					})
				})
			})
			// We are only interested in banked symbols
			.filter_map(|(name, loc)| match loc {
				gb_sym_file::Location::Banked(bank, addr) => Some((name, (bank, addr))),
				_ => None,
			});
		symfile.extend(symbols);
	}
	symfile
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

	let mut rom = Vec::<u8>::new();
	open_input(&rom_path).read_to_end(&mut rom).unwrap_or_else(|error| {
		eprintln!("Failed to read {rom_path}: {error}");
		exit(1);
	});
	if rom.len() < 0x4000 {
		rom.resize(0x4000, 0xFF);
	}

	let address_space = AddressSpace::with(&rom);
	
	let mut config_text = String::new();
	open_input(&config_path)
		.read_to_string(&mut config_text)
		.unwrap_or_else(|error| {
			eprintln!("Failed to read {config_path}: {error}");
			exit(1);
		});

	let symfile = read_symfile(&cli.symfile);
	let tests = read_config(&config_text, &symfile);

	let mut logger = Logger::new(cli.silent, &rom_path);

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
