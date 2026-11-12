//! How the RX C ABI counts R1-R4 (compiler/rustc_target/src/callconv/rx.rs).
//! CC-RX (the default) counts only the arguments placed in registers, so the
//! arguments after one that went on the stack still take the free registers;
//! GNURX (`llvm-abiname: "gnu"`) counts every argument, a structure on the
//! stack included. A structure goes in registers as integers (`i32` /
//! `[N x i32]`) or on the stack (`byval`).

//@ add-minicore
//@ revisions: ccrx gnu
//@[ccrx] compile-flags: --target rx-none-elf
//@[gnu] compile-flags: --target={{src-base}}/rx-gnu-abi.json -Z unstable-options
//@ needs-llvm-components: rx
//@ compile-flags: -Copt-level=0 -Cno-prepopulate-passes

#![feature(no_core, lang_items)]
#![crate_type = "lib"]
#![no_std]
#![no_core]

extern crate minicore;
use minicore::*;

#[repr(C)]
pub struct S4 {
    a: i32,
}
#[repr(C)]
pub struct S6 {
    s: [i16; 3],
}
#[repr(C)]
pub struct S12 {
    a: i32,
    b: i32,
    c: i32,
}
#[repr(C)]
pub struct S16 {
    a: [i32; 4],
}

// S12 does not fit after two ints and goes on the stack. CC-RX puts S4 in
// R3; GNURX's count is at 20, so S4 is on the stack too.
// CHECK-LABEL: @after_nofit(
// CHECK-SAME: i32 {{.*}}%a, i32 {{.*}}%b, ptr {{.*}}byval([12 x i8]) align 4 {{.*}}%c,
// ccrx-SAME: i32 %0)
// gnu-SAME: ptr {{.*}}byval([4 x i8]) align 4 {{.*}}%d)
#[no_mangle]
pub extern "C" fn after_nofit(a: i32, b: i32, c: S12, d: S4) {}

// S6 (6 bytes) is always on the stack. For CC-RX it takes no register and
// S16 fills R1-R4; for GNURX it counts 8 bytes and S16 no longer fits.
// CHECK-LABEL: @after_stack_only(
// CHECK-SAME: ptr {{.*}}byval([6 x i8]) align 2 {{.*}}%a,
// ccrx-SAME: [4 x i32] %0)
// gnu-SAME: ptr {{.*}}byval([16 x i8]) align 4 {{.*}}%b)
#[no_mangle]
pub extern "C" fn after_stack_only(a: S6, b: S16) {}

// After both have taken their registers nothing is left for S4: on the
// stack in both.
// CHECK-LABEL: @full(
// CHECK-SAME: [4 x i32] %0, ptr {{.*}}byval([4 x i8]) align 4 {{.*}}%b)
#[no_mangle]
pub extern "C" fn full(a: S16, b: S4) {}
