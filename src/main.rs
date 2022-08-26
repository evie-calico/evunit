use rgbunit::cpu;
use rgbunit::memory;
use std::env;
use std::process::exit;

fn main() {
	let args: Vec<String> = env::args().collect();
	if args.len() < 2 {
		eprintln!("usage: {} <rom path>", args[0]);
		exit(1);
	}

	let address_space = match memory::AddressSpace::open(&args[1]) {
		Ok(result) => result,
		Err(error) => {
			eprintln!("Failed to open {}: {}", args[1], error);
			exit(1);
		}
	};

	let mut cpu_state = cpu::State::new(address_space);

	loop {
		let complete = cpu_state.tick();
		println!("{cpu_state:#?}\n");
		if complete { break }
	}
}
