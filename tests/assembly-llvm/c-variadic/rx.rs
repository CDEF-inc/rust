//@ add-minicore
//@ assembly-output: emit-asm
//
//@ revisions: RX
//@ [RX] compile-flags: -Copt-level=3 --target rx-none-elf
//@ [RX] needs-llvm-components: rx
// CDEF-inc fork: c-variadic definitions are stable on RX, so there is no
// `c_variadic_experimental_arch` here.
#![feature(no_core, lang_items, intrinsics, rustc_attrs)]
#![no_core]
#![crate_type = "lib"]

// RX follows GCC's rule for variadic calls: the last named argument and all
// the unnamed ones go on the stack, each in 4-byte slots (i64 and f64 are
// 4-byte aligned), and `va_list` is a pointer to the next one. rustc uses
// LLVM's `va_arg`, as clang does. rx-rust's test/rust/variadic-smoke checks
// these against GCC-compiled C in both directions.

extern crate minicore;
use minicore::ffi::VaList;
use minicore::*;

#[unsafe(no_mangle)]
unsafe extern "C" fn read_i32(ap: &mut VaList<'_>) -> i32 {
    // CHECK-LABEL: _read_i32:
    //
    // RX: mov.l [r1], r2
    // RX-NEXT: add #3, r2
    // RX-NEXT: and #-4, r2
    // RX-NEXT: add #4, r2, r3
    // RX-NEXT: mov.l r3, [r1]
    // RX-NEXT: mov.l [r2], r1
    // RX-NEXT: rts
    ap.next_arg()
}

#[unsafe(no_mangle)]
unsafe extern "C" fn read_i64(ap: &mut VaList<'_>) -> i64 {
    // CHECK-LABEL: _read_i64:
    //
    // An i64 takes two slots, low word first, with no padding before it.
    // RX: mov.l [r1], r2
    // RX-NEXT: add #3, r2
    // RX-NEXT: and #-4, r2
    // RX-NEXT: add #4, r2, r3
    // RX-NEXT: mov.l r3, [r1]
    // RX-NEXT: mov.l [r2], r3
    // RX-NEXT: add #8, r2, r4
    // RX-NEXT: mov.l r4, [r1]
    // RX-NEXT: mov.l 4[r2], r2
    // RX-NEXT: mov.l r3, r1
    // RX-NEXT: rts
    ap.next_arg()
}

#[unsafe(no_mangle)]
unsafe extern "C" fn read_f64(ap: &mut VaList<'_>) -> f64 {
    // CHECK-LABEL: _read_f64:
    //
    // f64 is read as an i64 (64-bit doubles, soft-float ABI: R1:R2).
    // RX: mov.l [r1], r2
    // RX-NEXT: add #3, r2
    // RX-NEXT: and #-4, r2
    // RX-NEXT: add #4, r2, r3
    // RX-NEXT: mov.l r3, [r1]
    // RX-NEXT: mov.l [r2], r3
    // RX-NEXT: add #8, r2, r4
    // RX-NEXT: mov.l r4, [r1]
    // RX-NEXT: mov.l 4[r2], r2
    // RX-NEXT: mov.l r3, r1
    // RX-NEXT: rts
    ap.next_arg()
}

#[unsafe(no_mangle)]
unsafe extern "C" fn first_vararg(_n: i32, mut ap: ...) -> i32 {
    // CHECK-LABEL: _first_vararg:
    //
    // A definition: the last named argument is at 4[r0] on entry (above the
    // return address) and the first unnamed one at 8[r0], here 12[r0] after
    // the 4-byte `va_list` slot is allocated.
    // RX: add #-4, r0, r0
    // RX: mov.l 12[r0], r1
    // RX-NEXT: rtsd #4
    ap.next_arg()
}

unsafe extern "C" {
    fn callee(n: i32, ...);
}

#[unsafe(no_mangle)]
unsafe extern "C" fn call_variadic() {
    // CHECK-LABEL: _call_variadic:
    //
    // The named argument too goes on the stack, then the i32 and the i64.
    // (A CFI directive follows the stack adjustment.)
    // RX: add #-16, r0, r0
    // RX: mov.l #0, 12[r0]
    // RX-NEXT: mov.l #3, 8[r0]
    // RX-NEXT: mov.l #2, 4[r0]
    // RX-NEXT: mov.l #1, 0[r0]
    // RX-NEXT: bsr.a _callee
    // RX-NEXT: rtsd #16
    callee(1, 2i32, 3i64)
}

// A pointer is read as an i32 (the alias comes at the end of the output).
#[unsafe(no_mangle)]
unsafe extern "C" fn read_ptr(ap: &mut VaList<'_>) -> *const u8 {
    // RX: _read_ptr = _read_i32
    ap.next_arg()
}
