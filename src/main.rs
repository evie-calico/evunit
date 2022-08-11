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
	let mut cpu_state = cpu::State::new();
	let mut address_space;
	match memory::AddressSpace::open(&args[1]) {
		Ok(result) => address_space = result,
		Err(error) => {
			eprintln!("Failed to open {}: {}", args[1], error);
			exit(1);
		}
	}
	loop {
		let complete = cpu_state.tick(&mut address_space);
		if complete { break }
	}
	println!("{cpu_state:#?}");
}
