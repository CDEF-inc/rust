//! The Renesas RX C ABI, as implemented by GCC (rx-elf / GNURX) in
//! `gcc/config/rx/rx.cc` (`rx_function_arg`, `rx_function_arg_advance`,
//! `rx_return_in_memory`).
//!
//! Arguments fill R1-R4 by a running byte count, each argument rounded up to
//! 4 bytes. An argument goes in registers only if it fits entirely; the count
//! advances for every argument. Structures whose size is a multiple of 4 are
//! passed in registers like integers; other structures, and any structure that
//! does not fit, are copied to the stack at their natural alignment.
//! Structures of 1-16 bytes whose size is a multiple of 4 are returned in
//! R1-R4; others are returned in memory through a pointer passed in R15
//! (which does not count as an argument). In a variadic function the unnamed
//! arguments and the last named one are on the stack, where a structure is
//! always copied at its natural alignment (a structure of four chars that
//! follows one of three chars starts at offset 3).
//!
//! GCC does not promote char and short arguments: the caller leaves the upper
//! bits of the register undefined (and stores a stacked char in one byte), and
//! the callee extends the value itself (`movu.b r1, r1`). So integer arguments
//! get no `zeroext` / `signext`, and a `bool` travels as an `i8` that is 0 or
//! 1 (GCC's callee reads the low byte). Return values are extended to 32 bits
//! by the callee, as GCC does.
//!
//! The LLVM backend assigns the registers and stack slots with the same
//! running count, so this only decides between integer parts and `byval`.
//! clang (`clang/lib/CodeGen/Targets/RX.cpp`) makes the same choices.

use rustc_abi::{BackendRepr, HasDataLayout, TyAbiInterface};

use crate::callconv::{ArgAbi, FnAbi, Reg, Uniform};

/// Bytes of arguments that fit in R1-R4.
const ARG_REG_BYTES: u64 = 16;

fn is_rx_aggregate<Ty>(arg: &ArgAbi<'_, Ty>) -> bool {
    arg.layout.is_aggregate() || matches!(arg.layout.backend_repr, BackendRepr::SimdVector { .. })
}

fn classify_ret<Ty>(ret: &mut ArgAbi<'_, Ty>) {
    if is_rx_aggregate(ret) {
        let size = ret.layout.size.bytes();
        if size % 4 == 0 && size <= ARG_REG_BYTES {
            ret.cast_to(Uniform::new(Reg::i32(), ret.layout.size));
        } else {
            ret.make_indirect();
        }
    } else {
        ret.extend_integer_width_to(32);
    }
}

fn classify_arg<'a, Ty, C>(cx: &C, arg: &mut ArgAbi<'a, Ty>, on_stack: bool, used_bytes: &mut u64)
where
    Ty: TyAbiInterface<'a, C> + Copy,
    C: HasDataLayout,
{
    if arg.layout.pass_indirectly_in_non_rustic_abis(cx) {
        arg.make_indirect();
        *used_bytes += 4;
        return;
    }
    if arg.layout.is_zst() {
        return;
    }

    let size = arg.layout.size.bytes();
    if is_rx_aggregate(arg) {
        if !on_stack && size % 4 == 0 && *used_bytes + size <= ARG_REG_BYTES {
            arg.cast_to(Uniform::new(Reg::i32(), arg.layout.size));
        } else {
            // On the stack at the type's own alignment (a 3-byte struct of
            // chars may be followed by another one at offset 3); as
            // `[N x i32]` it would be 4-byte aligned.
            arg.pass_by_stack_offset(None);
        }
    } else if arg.layout.backend_repr.is_bool() {
        // GCC's callee reads the low byte, which must be 0 or 1. An `i1`
        // with no extension may carry garbage in bits 1-7, and `zeroext`
        // would make a Rust callee trust bits 8-31.
        arg.cast_to_maybe_noundef(Reg::i8(), cx);
    }
    // Other scalars: no extension, as GCC's callers do not extend.
    *used_bytes += size.next_multiple_of(4);
}

pub(crate) fn compute_abi_info<'a, Ty, C>(cx: &C, fn_abi: &mut FnAbi<'a, Ty>)
where
    Ty: TyAbiInterface<'a, C> + Copy,
    C: HasDataLayout,
{
    if !fn_abi.ret.is_ignore() {
        classify_ret(&mut fn_abi.ret);
    }

    // In a variadic function the last named argument and the unnamed ones
    // are on the stack.
    let first_on_stack = if fn_abi.c_variadic {
        (fn_abi.fixed_count as usize).saturating_sub(1)
    } else {
        usize::MAX
    };
    let mut used_bytes = 0;
    for (i, arg) in fn_abi.args.iter_mut().enumerate() {
        if arg.is_ignore() {
            continue;
        }
        classify_arg(cx, arg, i >= first_on_stack, &mut used_bytes);
    }
}
