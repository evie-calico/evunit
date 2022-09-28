//! This module parses [sym files](https://rgbds.gbdev.io/docs/sym).

use regex::Regex;

use std::collections::HashMap;
use std::io::BufRead;

#[derive(Debug, Default)]
pub struct Symbols {
	pub symbols: HashMap<String, (u16, u16)>,
}

impl Symbols {
	pub fn new() -> Self {
		Default::default()
	}

	pub fn from_sym_file<R: BufRead>(input: R) -> Result<Self, std::io::Error> {
		let re = Regex::new("[ \t]*([0-9a-fA-F]{2,}):([0-9a-fA-F]{4})[ \t]+([a-zA-Z_].*)").unwrap();

		let symbols = input
			.lines()
			.filter_map(|line| match line {
				Err(e) => Some(Err(e)),
				Ok(line) => re.captures(&line).map(|caps| {
					let bank = u16::from_str_radix(caps.get(1).unwrap().as_str(), 16).unwrap();
					let addr = u16::from_str_radix(caps.get(2).unwrap().as_str(), 16).unwrap();
					let name = String::from(caps.get(3).unwrap().as_str());
					Ok((name, (bank, addr)))
				}),
			})
			.collect::<Result<_, _>>()?;

		Ok(Self { symbols })
	}
}
