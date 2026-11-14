//@ add-minicore
//@ revisions: RELAX NORELAX
//@ compile-flags: --target rx-none-elf -Copt-level=3
//@ [RELAX] compile-flags: -C target-feature=+relax
//@ needs-llvm-components: rx

// The RX target feature `relax` (linker relaxation hints): rustc knows it
// (`cfg(target_feature = "relax")`) and passes it on to LLVM, which then
// emits the hints. It is off by default.

#![crate_type = "lib"]
#![feature(no_core, rx_target_feature)]
#![no_core]

extern crate minicore;
use minicore::*;

// CHECK-LABEL: define{{.*}} @always
#[no_mangle]
pub fn always() -> u32 {
    1
}

// RELAX: define{{.*}} @only_with_relax
// NORELAX-NOT: @only_with_relax
#[cfg(target_feature = "relax")]
#[no_mangle]
pub fn only_with_relax() -> u32 {
    2
}

// RELAX: attributes #{{[0-9]+}} = {{.*}}"target-features"="{{[^"]*}}+relax
// NORELAX-NOT: +relax
