# Behistun

An m68020 linux to x86_64 linux userspace interpreter.

## Running

Run your m68020 compiled binary, e.g.

```sh
$ m68k-unknown-linux-uclibc-gcc -static -msoft-float -O0 examples/cat.c -o
cat
$ file cat
cat: ELF 32-bit MSB executable, Motorola m68k, 68020, version 1 (SYSV), statically linked, with debug_info, not stripped
$ file $(which head)
/usr/bin/head: ELF 64-bit LSB pie executable, x86-64, version 1 (SYSV), dynamically linked, interpreter /lib64/ld-linux-x86-64.so.2, BuildID[sha1]=3aa5a70d3e5aed49c315c6c44cb2f9ab2dde567b, for GNU/Linux 4.4.0, stripped
# run it in a pipeline with your usual binaries
$ cargo r -q -- cat README.md | head -n 3
# Behistun

An m68020 linux to x86_64 linux userspace interpreter.
```

Check around the `examples/` dir for some examples, or run some of the
tests in `test-files`.

## Features

A good chunk of linux syscalls are supported. About 300 or so are currently
supported. As well, all of the 68000 and most of the 68020 instruction
set are supported, except for the coprocessor instructions.

## Credits

Decoding instructions couldn't be done without reading this great guide
at [goldencrystal.free.fr](http://goldencrystal.free.fr/M68kOpcodes-v2.3.pdf) 
that explains how each 68000 opcode is read and deconstructed. As well,
qemu for having a reference implementation to easily test against and
[crosstool](https://crosstool-ng.github.io/) for building compilers + 
[uclibc-ng](https://uclibc-ng.org/) for having a libc that supports
68020.

## Developing

To develop on this project, you'll need a few things.

First, an m68020 compatible compiler. This is **really** difficult. At
first, I thought you could use
[`crosstool`](https://crosstool-ng.github.io/) to compile an
m68k-unknown-linux-gnu-* toolchain that you can use to produce ELF
binaries. It turns out that's not the case, because by default, glibc is
compiled with support for hardware floats. The m68020, 68010, and 68000
don't support hardware floats, so glibc is out. Musl has the same
problem, where setjmp/longjmp reset the floating point registers, which
the m68020 doesn't have, so it also doesn't compile. That leaves uclibc.
You can compile uclibc while targeting the m68020 -- you'll need that
toolchain, explicitly compiled for 68020 support to produce ELF binaries
that behistun can run. It's basically hardcoded in the project's
makefiles that you need this.

If you only plan to use behistun for assembly, then
`m68k-unknown-linux-gnu-` or `m68k-unknown-linux-musl` will suffice,
since floats won't get into your assembly code unless you explicitly
write them. However some libc routines will have floats because of your
compiler, so you won't be able to use libc if you go down this route.

Next, for testing, you'll need `qemu-m68k` as a reference implementation
to test against. Behistun breaks from `qemu` a bit in terms of what it
does, so it's not a 1:1 copy, but it's useful to have an oracle to test
against.

Finally, an x86_64 linux host. Behistun supports syscall translation
from m68k to x86_64, so it's meant to be run on an x86_64 host. There
might be other backends someday, but even trying to translate all the
syscalls is quite hard, since quite a few syscalls have to be supported
in the interpreter itself.

## Testing

Test files live in `test-files`, `test-integration`, and `test-csmith`.
The first directory has asm and c files that test for equality of
stdout, stderr, and return code against `qemu-m68k`. If you want
behavior that lines up with qemu, write your test there. For
integration, this is for tests for functionality where it doesn't line
up with `qemu`, say implementing `atomic_barrier`, which qemu doesn't
have. Since it doesn't support it, we can't test our implementation
against it, so test files for those features go here. Finally, csmith
requires the `csmith` program, which generates random C programs. I
generated 100 of them and made sure they could all compile and generate
the same output as `qemu`.

## Architecture

The architecture is quite simple. The project first uses `goblin` to
parse an ELF binary, and then decodes the instructions in the executable
sections of the binary. The instruction decoder lives in
`src/decoder/*.rs`. Then, those instructions are fed to the interpreter,
located at `src/cpu.rs`, which executes them. Most of the code in
`src/cpu.rs` focuses on executing instructions, and there's a syscall
handler in there that handles syscalls as well by translating to the
host, not implementing them (returning -1), or not supporting them and
returning an error (those I haven't gotten to).
