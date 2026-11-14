# `rx-none-elf`

**Tier: 3**

Bare-metal Renesas RX microcontrollers (RX100, RX200, RX600 and RX700 series:
RXv1, RXv2, RXv3 and RXv3e cores).

## Target maintainers

[@taka-intpe](https://github.com/taka-intpe)

## Requirements

This target is cross-compiled; there is no host tooling. It supports `core`
(and `alloc` with a user-provided allocator) but not `std`.

The RX backend is not part of upstream LLVM. Building this target needs an
LLVM with the experimental RX backend enabled
(`-DLLVM_EXPERIMENTAL_TARGETS_TO_BUILD=RX`).

The linker is `rust-lld`, which rustc calls directly. Upstream LLD has no RX
support either: it needs the RX port of the same LLVM fork, built with
`-DLLVM_ENABLE_PROJECTS=lld` and installed as `rust-lld` in the sysroot's
`lib/rustlib/<host>/bin`. It links as GNU ld from binutils for `rx-elf` does,
which `-C linker=rx-elf-ld` selects instead. Runtime routines that libgcc
would otherwise provide come from `compiler_builtins`.

The default C ABI is that of Renesas CC-RX. GCC for `rx-elf` with
`-m64bit-doubles` and Renesas GNURX use the same ABI except for one rule,
how argument registers are counted (below). Both ABIs share these rules:

* little-endian, 32-bit pointers; `i64` and `f64` are 4-byte aligned;
  `double` is 64 bits;
* C symbols carry a leading underscore;
* arguments fill R1-R4 by a byte count, each rounded up to 4 bytes; an
  argument goes in registers only if it fits entirely, otherwise on the
  stack; the last named argument and all variadic arguments of a variadic
  function go on the stack;
* aggregates of 1-16 bytes whose size is a multiple of 4 are returned in
  R1-R4, others through a pointer in R15;
* R6-R13 are callee-saved, R0 is the stack pointer.

CC-RX counts only the arguments it places in registers, so after an argument
that went on the stack, a later argument that fits still goes in a free
register. GCC and GNURX count every argument, so all the arguments after
that one go on the stack. Calls without such an argument are the same in
both ABIs. To call, or be called by, C compiled with GCC or GNURX, use a
custom target: the JSON of `rx-none-elf` (`rustc -Z unstable-options
--print target-spec-json --target rx-none-elf`) with `"llvm-abiname":
"gnu"`. clang selects the same ABI with `-mabi=gnu`.

Floating point uses soft-float routines (`compiler_builtins`, fully IEEE)
by default. With `-C target-feature=+flush-denormals` (unstable
`rx_target_feature`), `f32` arithmetic, compares and `as` conversions use
the FPU, and on `rxv3` / `rxv3e` `f64` uses the double-precision FPU (DFPU).
The results are IEEE for NaNs, infinities, signed zeros and rounding; only
denormal inputs and results become zero (FPSW.DN / DPSW.DDN, which the
startup code sets). The FPU is about 28 times faster than soft-float on an
RX72M. GCC, by contrast, uses the FPU only with `-ffinite-math-only`.

The default CPU is `rxv3`; pass `-C target-cpu=rxv1` or `rxv2` for older
cores. Code generation uses the instructions of the selected core: the RXv2
3-operand and FPU forms, `emul` / `emulu`, `movli` / `movco` for atomics,
and the DFPU on RXv3.

Atomics are available up to 32 bits (`max_atomic_width`), including
compare-and-swap and read-modify-write operations. On RXv2 and later they are
a `movli`/`movco` loop; on RXv1, which has no such instructions, the
compare-and-swap runs with interrupts masked (`clrpsw i`), which is atomic
only in supervisor mode. Panics abort; there is no unwinding.

## Building the target

Use an LLVM with the RX backend through `llvm-config` and add the target to
`bootstrap.toml`:

```toml
[build]
target = ["x86_64-unknown-linux-gnu", "rx-none-elf"]

[target.x86_64-unknown-linux-gnu]
llvm-config = "/path/to/llvm/bin/llvm-config"
```

Alternatively build only the compiler and use `-Zbuild-std`.

## Building Rust programs

Rust does not ship pre-compiled artifacts for this target. Use
`-Zbuild-std`, for example in `.cargo/config.toml`:

```toml
[build]
target = "rx-none-elf"

[target.rx-none-elf]
rustflags = ["-C", "link-arg=-Tlink.x"]

[unstable]
build-std = ["core"]
build-std-features = ["compiler-builtins-mem"]
```

The linker script and startup code (setting up the stack and `INTB`,
initialising `.data` and `.bss`, the vector tables) are not provided by rustc.

Inline assembly is supported with `#![feature(asm_experimental_arch)]`: the
`reg` class covers R1-R15, and `clobber_abi("C")` clobbers R1-R5, R14 and R15.

## Testing

There is no support for running the rustc test suite on this target.
Programs can be run on QEMU's RX62N board (`qemu-system-rx -M
gdbsim-r5f562n8`), which implements RXv1.

## Cross-compilation toolchains and C code

C code is compiled with GCC for `rx-elf` (`-m64bit-doubles`). Upstream GCC
knows RXv1 CPUs (`-mcpu=rx600` etc.); Renesas GNURX adds `-misa=v2|v3` and
`-mdfpu`.
