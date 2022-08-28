use regex::Regex;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;

pub struct Symfile {
	pub symbols: HashMap<String, (u16, u16)>
}

impl Symfile {
	pub fn new() -> Symfile { Symfile { symbols: HashMap::new() } }

	pub fn open(path: &String) -> Result<Symfile, std::io::Error> {
		let mut symfile = Symfile::new();
		let lines = BufReader::new(File::open(path)?).lines();
		let re = Regex::new("[ \t]*([0-9a-fA-F]{2,}):([0-9a-fA-F]{4})[ \t]+([a-zA-Z_].*)").unwrap();

		for line in lines {
			let line = line?;
			if let Some(caps) = re.captures(&line) {
				let bank = u16::from_str_radix(caps.get(1).unwrap().as_str(), 16).unwrap();
				let addr = u16::from_str_radix(caps.get(2).unwrap().as_str(), 16).unwrap();
				let name = String::from(caps.get(3).unwrap().as_str());
				symfile.symbols.insert(name, (bank, addr));
			}
		}

		Ok(symfile)
	}
}
