# rgbunit

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
- a
- b
- c
- d
- e
- h
- l
- z.f
- z.n
- z.h
- z.c
- pc
- sp

You can assign an integer to any of these (`0x` for hex), or a label if you have a symfile loaded

> symfiles are not yet supported

This means that to determine which functions should run, you can assign a label to `pc`.

Sometimes you have configurations which should apply to all tests, like a global variable or the stack pointer.
Any configurations at the top of the file (before a heading) are global and apply to all tests.

```toml
sp = wStack.end

[my-test]
pc = MyTest
a = 42
[my-test.result]
b = 42
```

If the test result is absent, the test will always pass unless it crashes.
