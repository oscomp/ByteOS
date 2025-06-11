use log::warn;
use syscalls::Errno;
use xmas_elf::{program::Type, sections::SectionData, ElfFile};

pub trait ElfExtra {
    fn get_ph_addr(&self) -> Result<u64, Errno>;
    fn relocate(&self, base: usize) -> Result<usize, &str>;
}

impl ElfExtra for ElfFile<'_> {
    // 获取elf加载需要的内存大小
    fn get_ph_addr(&self) -> Result<u64, Errno> {
        if let Some(phdr) = self
            .program_iter()
            .find(|ph| ph.get_type() == Ok(Type::Phdr))
        {
            // if phdr exists in program header, use it
            Ok(phdr.virtual_addr())
        } else if let Some(elf_addr) = self
            .program_iter()
            .find(|ph| ph.get_type() == Ok(Type::Load) && ph.offset() == 0)
        {
            // otherwise, check if elf is loaded from the beginning, then phdr can be inferred.
            Ok(elf_addr.virtual_addr() + self.header.pt2.ph_offset())
        } else {
            warn!("elf: no phdr found, tls might not work");
            Err(Errno::EBADF)
        }
    }

    fn relocate(&self, base: usize) -> Result<usize, &str> {
        let data = self
            .find_section_by_name(".rela.dyn")
            .ok_or(".rela.dyn not found")?
            .get_data(self)
            .map_err(|_| "corrupted .rela.dyn")?;
        match data {
            SectionData::Rela64(_) => Ok(base),
            _ => return Err("bad .rela.dyn"),
        }
    }
}
