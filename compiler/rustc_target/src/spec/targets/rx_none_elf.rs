use crate::spec::{
    Arch, Cc, LinkerFlavor, Lld, PanicStrategy, RelocModel, Target, TargetMetadata, TargetOptions,
};

pub(crate) fn target() -> Target {
    Target {
        llvm_target: "rx-none-elf".into(),
        metadata: TargetMetadata {
            description: Some("Renesas RX microcontrollers (RXv1-RXv3e)".into()),
            tier: Some(3),
            host_tools: Some(false),
            std: Some(false),
        },
        pointer_width: 32,
        // `m:o`: C symbols get the `_` prefix GCC uses on RX. i64 and f64 are
        // 4-byte aligned.
        data_layout: "e-m:o-p:32:32-i64:32-f64:32-n8:16:32-S32".into(),
        arch: Arch::Rx,

        options: TargetOptions {
            cpu: "rxv3".into(),
            // `double` is 64 bits, as with GCC's -m64bit-doubles (recorded in
            // the ELF header, which GNU ld checks).
            features: "+64bit-doubles".into(),

            // There is no RX support in LLD, and no libc: call GNU ld directly.
            // compiler_builtins provides the runtime routines libgcc would.
            linker: Some("rx-elf-ld".into()),
            linker_flavor: LinkerFlavor::Gnu(Cc::No, Lld::No),

            // Aligned loads and stores of up to 32 bits are atomic (all parts
            // are single-core). Compare-and-swap is a MOVLI/MOVCO loop on
            // RXv2 and later, and runs with interrupts masked on RXv1.
            max_atomic_width: Some(32),
            atomic_cas: true,

            // No unwinder; GNU as for rx-elf does not accept CFI directives.
            panic_strategy: PanicStrategy::Abort,
            relocation_model: RelocModel::Static,
            emit_debug_gdb_scripts: false,
            eh_frame_header: false,

            ..Default::default()
        },
    }
}
