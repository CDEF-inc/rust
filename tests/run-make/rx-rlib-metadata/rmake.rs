//! The metadata of an rx-none-elf rlib (`lib.rmeta`) is an RX ELF object whose e_flags match
//! the code objects', except the string instruction bits, which depend on the code. Without
//! `Architecture::Rx` in object, rustc stored the metadata raw.

//@ needs-llvm-components: rx
//@ ignore-backends: gcc

use object::elf;
use object::read::archive::ArchiveFile;
use object::read::elf::FileHeader;
use run_make_support::{object, rfs, rustc_minicore};

fn check(cpu: &str, flags: &[&str], expected: u32) {
    let name: String = flags.concat().chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    let rlib = format!("libminicore-{cpu}-{name}.rlib");
    rustc_minicore().target("rx-none-elf").target_cpu(cpu).args(flags).output(&rlib).run();

    let data = rfs::read(&rlib);
    let archive = ArchiveFile::parse(&*data).unwrap();
    let mut metadata = None;
    let mut code = Vec::new();
    for member in archive.members() {
        let member = member.unwrap();
        let name = String::from_utf8_lossy(member.name()).into_owned();
        let member_data = member.data(&*data).unwrap();
        let header = elf::FileHeader32::<object::Endianness>::parse(member_data)
            .unwrap_or_else(|e| panic!("{rlib}: {name} is not an ELF32 object: {e}"));
        let endian = header.endian().unwrap();
        assert_eq!(header.e_machine(endian), elf::EM_RX, "{rlib}: {name}");
        let e_flags = header.e_flags(endian);
        if name == "lib.rmeta" {
            metadata = Some(e_flags);
        } else if name.ends_with(".o") {
            code.push((name, e_flags));
        }
    }

    let metadata = metadata.unwrap_or_else(|| panic!("{rlib}: no lib.rmeta"));
    assert_eq!(metadata, expected, "{rlib}: lib.rmeta e_flags {metadata:#x}");
    assert!(!code.is_empty(), "{rlib}: no code object");
    for (name, e_flags) in code {
        assert_eq!(
            e_flags & !elf::EF_RX_SINSNS_MASK,
            metadata,
            "{rlib}: {name} e_flags {e_flags:#x}"
        );
    }
}

fn main() {
    // rx-none-elf has `+64bit-doubles` and natural argument alignment (EF_RX_ABI).
    let base = elf::EF_RX_ABI | elf::EF_RX_64BIT_DOUBLES;
    check("rxv1", &[], base);
    check("rxv2", &[], base | elf::EF_RX_V2);
    check("rxv3", &[], base | elf::EF_RX_V3);
    check("rxv3e", &[], base | elf::EF_RX_V3);
    check("rxv3", &["-Ctarget-feature=-64bit-doubles"], elf::EF_RX_ABI | elf::EF_RX_V3);
    check("rxv3", &["-Zllvm-target-feature=-64bit-doubles"], elf::EF_RX_ABI | elf::EF_RX_V3);
    // `-Z llvm-target-feature` goes to LLVM after `-C target-feature`.
    check(
        "rxv3",
        &["-Ctarget-feature=-64bit-doubles", "-Zllvm-target-feature=+64bit-doubles"],
        base | elf::EF_RX_V3,
    );
}
