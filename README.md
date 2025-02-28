# evunit

This is a unit testing application for Game Boy roms.
It only contains a CPU emulator; no PPU, memory mapper, or I/O.
By using real binaries as input, you can run unit tests on your finished ROM without any need to rebuild.

The command-line tool loads test configurations from TOML files.
You can also use it as a Rust library, and configure your tests from Rust code.

[Changelog](./CHANGELOG.md)

## Installing

`cargo install evunit`

## Configuring a test

Within the test config you can create a heading for each test you want to run, and assign default and expected values for registers.
The first heading (for example, `add-one`) determines the initial state, while the "`.result`" heading (for example, `add-one.result`) describes the expected result.
If the expected result does not match the final state, the test will fail.

```toml
[add-one]
b = 1
[add-one.result]
a = 2

[add-two]
b = 2
[add-two.result]
a = 3
```

You can assign any cpu register to an integer.
In addition, 16-bit registers may be assigned a quoted label if a symfile is loaded.
To determine which function should run in each test, assign a label to `pc`.
Possible registers are:
- `a`
- `b`
- `c`
- `d`
- `e`
- `h`
- `l`
- `bc`
- `de`
- `hl`
- `pc`
- `sp`

In addition the flags can be assigned to either `true` or `false`.
Possible flags are:
- `f.z`
- `f.n`
- `f.h`
- `f.c`

Note that the flags must be *quoted* in the config file because of the dot:
```toml
"f.z" = false
```

Finally, memory can be assigned a value in the config file by surrounding a label name or address in square brackets.
You can either assign an 8-bit integer, a string*, or an array of either.
Like the flags, memory addresses must be quoted because of the square brackets:

```toml
# Writes a string to wString, followed by a 0
"[wString]" = ["Hello, world!", 0]

# Writes a series of bytes to WRAM0
"[0xC000]" = [ 0x01, 0x02, 0x03, 0x04 ]
```

\* = Note that string are converted to their ASCII representation.
Strings containing Non-ASCII characters will return errors.

## Global configurations

Sometimes you have configurations which should apply to all tests, like a global variable or the stack pointer.
Any configurations at the top of the file (before a heading) are global and apply to all tests.

```toml
sp = "wStack.end"

[my-test]
pc = "MyTest"
a = 42
[my-test.result]
b = 42
```

If the test result is absent, the test will always pass unless it crashes.

Creating an exhaustive set of tests by hand might be tedious, so remember that you an always generate tests in Rust by using `evunit` as a library:.

```rust,ignore
use evunit::prelude::*;
use std::{path::Path, process::exit};

let rom = "bin.gb";
let sym = Some(Path::new("bin.sym"));
let symfile = read_symfile(sym);

let mut tests = Vec::new();

for i in 0..8 {
	let mut test = TestConfig::new(format!("my-test{i}"));

	// Initial state
	test.initial = Registers::new()
		.with_pc(symfile["GetBitA"].0 as u16)
		.with_a(i);

	// Expected state
	test.result = Some(Registers::new()
		.with_a(i));

	tests.push(test);
}

let result = run_tests(&rom, &tests, SilenceLevel::Passing);

if result.is_err() {
	exit(1);
}
```

Then pipe this into evunit.
You can use `-` to read from stdin.

```sh
./config_generator | evunit -c - bin/rom.gb
```

And you can always use `cat` to add a handwritten file into the mix.

```sh
./config_generator | cat config.toml - | evunit -c - bin/rom.gb
```

## Terminating a test

A test is complete when either a crash address is reached, the test times out, or `pc` is equal to the `caller` specified in the config file (default is `0xFFFF`).
evunit pushes the `caller` value to the stack before running your test, meaning that in most scenarios a `ret` will end the test.
When the `caller` value is successfully reached, evunit checks to see if the result matches what was expected.

## Configuration options

In addition to registers, there are a few other options you can configure.
All of these can be configured globally as well as per-test.

### caller

Sets the caller address.
This address is pushed to the stack when a test begins, allowing `ret` to end the test.

```toml
caller = "Main"
```

By default, `caller` is set to `0xFFFF`.

### crash

Marks an address as a "crash", causing the test to fail if `pc` reaches it.
This is useful for crash handler functions such as `rst $38`

```toml
crash = 0x38
```

An array of values can also be used.

```toml
crash = [0x38, "crash"]
```

### enable-breakpoints

Enables or disables printing register info after executing `ld b, b` and `ld d, d`.
Enabled by default.
This configuration can only be used globally.

```toml
enable-breakpoints = true
enable-breakpoints = false
```

### exit

Marks an address as an "exit", causing the test to end if `pc` reaches it.
The results will then be verified.

```toml
exit = "SomeFunction.exit"
```

An array of values can also be used.

### timeout

Sets the maximum number of cycles before a test fails.
This is useful if you have code that tends to get stuck in infinite loops, or code which can take an extremely long time to complete.
The default value is 65536.

```toml
timeout = 65536
```

### stack

Specifies data to be pushed onto the stack prior to a test being ran (just before `caller` is pushed).
Some functions expect their arguments to be on the stack,
so this option allows to do this.
You can either assign an 8-bit integer, a string, or an array of either.

```toml
stack = [ 0x04, 0x71, 0xff, "\n" ]
```

Do note that values are pushed to the stack in reverse.
As an example, the initial values on the stack for the example above look like the following (assuming `sp` = `0xD000`):

```
| Address | Data         |
| ------- | ------------ |
| 0xCFFF  | 0x0A         |
| 0xCFFF  | 0xFF         |
| 0xCFFF  | 0x71         |
| 0xCFFE  | 0x04         |
| 0xCFFD  | high(caller) |
| 0xCFFC  | low(caller)  |
```

## Diagnosing failures

When a test fails, it outputs some cpu registers depending on the failure reason to help you diagnose the issue.
However, sometimes you need to check the state of memory as well; this can be accomplished with the `--dump-dir` (`-d`) flag.
Pass a directory to this flag and when any test fails a text dump of memory will be placed in the provided directory.

```bash
evunit -c fail.toml -d dump/ rom.gb
```

The dump is simply a giant list of bytes, with headers for each memory type:

```not_rust
[WRAM 0]
[WRAM]
0xc000: 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00
0xc010: 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00
0xc020: 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00
0xc030: 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00
0xc040: 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00
0xc050: 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00
0xc060: 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00
...
```
