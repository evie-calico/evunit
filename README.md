# evunit

This is a unit testing application for Game Boy roms.
It includes a CPU emulator, and loads test configurations from TOML files.

## Configuring a test

Within the test config you can create a heading for each test you want to run, and assign default and expected values for registers

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

The values you can initialize are:
- a, b, c, d, e, h, l (8-bit registers)
- bc, de, hl, pc, sp (16-bit registers)
- z.f, z.n, z.h, z.c (Boolean flags)

You can assign an integer (or `true`/`false` for flags) to any of these (`0x` for hex), or a label if you have a symfile loaded.
To determine which functions should run, you can assign a label to `pc`.

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

## Additional configuration options

In addition to registers, there are a few other options you can configure.
All of these can be configured globally as well as per-test.

### crash

Marks an address as a "crash", causing the test to fail if `pc` reaches it.
This is useful for crash handler functions such as `rst $38`

```toml
crash = 0x38
crash = "crash"
```

### enable-breakpoints

Enables or disables printing register info after executing `ld b, b` and `ld d, d`.
Enabled by default.
This configuration can only be used globally.

```toml
enable-breakpoints = true
enable-breakpoints = false
```

### timeout

Sets the maximum number of cycles before a test fails.
This is useful if you have code that tends to get stuck in infinite loops, or code which can take an extremely long time to complete.
The default value is 65536.

```toml
timeout = 65536
```

## Diagnosing failures

When a test fails, it outputs some cpu registers depending on the failure reason to help you diagnose the issue.
However, sometimes you need to check the state of memory as well; this can be accomplished with the `--dump-dir` (`-d`) flag.
Pass a directory to this flag and when any test fails a text dump of memory will be placed in the provided directory.

```bash
evunit -c fail.toml -d dump/ rom.gb
```

The dump is simply a giant list of bytes, with headers for each memory type:

```
[WRAM 0]
0xc000: 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0
0xc010: 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0
0xc020: 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0
0xc030: 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0
0xc040: 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0
0xc050: 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0
0xc060: 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0 0x0
...
```
