//! The Renesas RX C ABI: CC-RX's (the default; CC-RX manual 9.1) or GNURX's
//! (`llvm-abiname: "gnu"` in a target JSON; `gcc/config/rx/rx.cc`:
//! `rx_function_arg`, `rx_function_arg_advance`, `rx_return_in_memory`).
//!
//! Arguments fill R1-R4 by a byte count, each argument rounded up to 4
//! bytes. An argument goes in registers only if it fits entirely. CC-RX
//! advances the count only for an argument placed in registers, so the
//! arguments after one that went on the stack still fill the free
//! registers; GNURX advances it for every argument. Structures whose size is
//! a multiple of 4 are passed in registers like integers; other structures,
//! and any structure that does not fit, are copied to the stack at their
//! natural alignment. Structures of 1-16 bytes whose size is a multiple of 4
//! are returned in R1-R4; others are returned in memory through a pointer
//! passed in R15 (which does not count as an argument). In a variadic
//! function the unnamed arguments and the last named one are on the stack,
//! where a structure is always copied at its natural alignment (a structure
//! of four chars that follows one of three chars starts at offset 3).
//!
//! char, short and bool arguments are extended to 32 bits by the caller
//! (`zeroext` / `signext`), which CC-RX's callee relies on; the LLVM backend
//! extends them again in the callee, since GNURX's caller leaves the upper
//! bits undefined. On the stack they take their own size. Return values are
//! extended by the callee in both.
//!
//! The LLVM backend assigns the registers and stack slots with the same
//! count (its `-target-abi` is `llvm_abiname`), so this only decides between
//! integer parts and `byval`. clang (`clang/lib/CodeGen/Targets/RX.cpp`)
//! makes the same choices.

use rustc_abi::{BackendRepr, HasDataLayout, TyAbiInterface};

use crate::callconv::{ArgAbi, FnAbi, Reg, Uniform};
use crate::spec::{HasTargetSpec, LlvmAbi};

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

fn classify_arg<'a, Ty, C>(
    cx: &C,
    arg: &mut ArgAbi<'a, Ty>,
    on_stack: bool,
    gnu: bool,
    used_bytes: &mut u64,
) where
    Ty: TyAbiInterface<'a, C> + Copy,
    C: HasDataLayout,
{
    // A pointer to the value takes 4 bytes.
    let indirect = arg.layout.pass_indirectly_in_non_rustic_abis(cx);
    if !indirect && arg.layout.is_zst() {
        return;
    }
    let size = if indirect { 4 } else { arg.layout.size.bytes() };
    let mut in_regs = !on_stack && *used_bytes + size.next_multiple_of(4) <= ARG_REG_BYTES;

    if indirect {
        arg.make_indirect();
    } else if is_rx_aggregate(arg) {
        in_regs &= size % 4 == 0;
        if in_regs {
            arg.cast_to(Uniform::new(Reg::i32(), arg.layout.size));
        } else {
            // On the stack at the type's own alignment (a 3-byte struct of
            // chars may be followed by another one at offset 3); as
            // `[N x i32]` it would be 4-byte aligned.
            arg.pass_by_stack_offset(None);
        }
    } else {
        // char, short and bool: extended by the caller (the backend extends
        // them again in the callee). A scalar goes in registers whenever it
        // fits; the backend decides.
        arg.extend_integer_width_to(32);
    }
    // CC-RX counts only the arguments placed in registers, GNURX all.
    if in_regs || gnu {
        *used_bytes += size.next_multiple_of(4);
    }
}

pub(crate) fn compute_abi_info<'a, Ty, C>(cx: &C, fn_abi: &mut FnAbi<'a, Ty>)
where
    Ty: TyAbiInterface<'a, C> + Copy,
    C: HasDataLayout + HasTargetSpec,
{
    if !fn_abi.ret.is_ignore() {
        classify_ret(&mut fn_abi.ret);
    }

    let gnu = cx.target_spec().llvm_abiname == LlvmAbi::Gnu;
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
        classify_arg(cx, arg, i >= first_on_stack, gnu, &mut used_bytes);
    }
}
