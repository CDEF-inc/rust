use std::fmt;

use rustc_span::Symbol;

use super::{InlineAsmArch, InlineAsmType, ModifierInfo};

def_reg_class! {
    Rx RxInlineAsmRegClass {
        reg,
    }
}

impl RxInlineAsmRegClass {
    pub fn valid_modifiers(self, _arch: super::InlineAsmArch) -> &'static [char] {
        &[]
    }

    pub fn suggest_class(self, _arch: InlineAsmArch, _ty: InlineAsmType) -> Option<Self> {
        None
    }

    pub fn suggest_modifier(
        self,
        _arch: InlineAsmArch,
        _ty: InlineAsmType,
    ) -> Option<ModifierInfo> {
        None
    }

    pub fn default_modifier(self, _arch: InlineAsmArch) -> Option<ModifierInfo> {
        None
    }

    pub fn supported_types(
        self,
        _arch: InlineAsmArch,
    ) -> &'static [(InlineAsmType, Option<Symbol>)] {
        // f32 lives in general-purpose registers (the RX FPU works on them).
        types! { _: I8, I16, I32, F32; }
    }
}

// R0 is the stack pointer (LLVM's RXRegisterInfo::getReservedRegs); the
// backend uses no frame pointer.
def_regs! {
    Rx RxInlineAsmReg RxInlineAsmRegClass {
        r1: reg = ["r1"],
        r2: reg = ["r2"],
        r3: reg = ["r3"],
        r4: reg = ["r4"],
        r5: reg = ["r5"],
        r6: reg = ["r6"],
        r7: reg = ["r7"],
        r8: reg = ["r8"],
        r9: reg = ["r9"],
        r10: reg = ["r10"],
        r11: reg = ["r11"],
        r12: reg = ["r12"],
        r13: reg = ["r13"],
        r14: reg = ["r14"],
        r15: reg = ["r15"],

        #error = ["r0", "sp"] =>
            "the stack pointer cannot be used as an operand for inline asm",
    }
}

impl RxInlineAsmReg {
    pub fn emit(
        self,
        out: &mut dyn fmt::Write,
        _arch: InlineAsmArch,
        _modifier: Option<char>,
    ) -> fmt::Result {
        out.write_str(self.name())
    }
}
