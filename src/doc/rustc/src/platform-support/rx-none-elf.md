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

LLD has no RX support, so linking uses GNU ld from binutils for `rx-elf`
(`rx-elf-ld`), which rustc calls directly. Runtime routines that libgcc would
otherwise provide come from `compiler_builtins`.

The ABI follows GCC for `rx-elf` with `-m64bit-doubles`, so Rust code can call
and be called by C compiled with GCC or Renesas GNURX:

* little-endian, 32-bit pointers; `i64` and `f64` are 4-byte aligned;
  `double` is 64 bits;
* C symbols carry a leading underscore;
* the first 16 bytes of arguments go in R1-R4, the rest on the stack; the
  last named argument and all variadic arguments of a variadic function go on
  the stack;
* aggregates of 1-16 bytes whose size is a multiple of 4 are returned in
  R1-R4, others through a pointer in R15;
* R6-R13 are callee-saved, R0 is the stack pointer.

Floating point uses soft-float routines: the RX FPU does not handle NaNs,
infinities or denormals the IEEE way (GCC enables `-ffinite-math-only`
whenever it uses it). With the LLVM `fpu` feature, which the `rxv2`, `rxv3`
and `rxv3e` CPUs enable, `f32` operations that LLVM may assume free of NaNs
and infinities (fast-math, e.g. `core::intrinsics::fadd_fast`) in functions
that allow denormals to be flushed (LLVM's `denormal_fpenv(preservesign)`,
which rustc never sets, so in Rust code this does not happen) and integer to
`f32` conversions use the FPU; everything else stays soft-float. Likewise,
with the `dfpu` feature (`rxv3`, `rxv3e`), fast-math `f64` arithmetic and
comparisons and integer to `f64` conversions use the double-precision FPU
(through DR0 / DR1). Values are passed in general registers either way. rustc does not know the RX target
features, so `-C target-feature=-fpu` is accepted with a warning.

The default CPU is `rxv3`; pass `-C target-cpu=rxv1` or `rxv2` for older
cores.

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
