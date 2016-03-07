
use memory::Frame;
use multiboot2::{ElfSection, ELF_SECTION_ALLOCATED,
                 ELF_SECTION_WRITABLE, ELF_SECTION_EXECUTABLE};


pub struct Entry(u64);

impl Entry {
    pub fn is_unused(&self) -> bool {
        self.0 == 0
    }

    pub fn set_unused(&mut self) {
        self.0 = 0;
    }

    pub fn flags(&self) -> EntryFlags {
        EntryFlags::from_bits_truncate(self.0)
    }

    pub fn pointed_frame(&self) -> Option<Frame> {
        if self.flags().contains(PRESENT) {
            Some(Frame::containing_address(self.0 as usize & 0x000f_ffff_ffff_f000))
        } else {
            None
        }
    }

    pub fn set(&mut self, frame: Frame, flags: EntryFlags) {
        assert!(frame.start_address() & !0x000f_ffff_ffff_f000 == 0);
        self.0 = (frame.start_address() as u64) | flags.bits();
    }
}

bitflags! {
    flags EntryFlags: u64 {
        const PRESENT         = 1 << 0,
        const WRITABLE        = 1 << 1,
        const USER_ACCESSIBLE = 1 << 2,
        const WRITE_THROUGH   = 1 << 3,
        const NO_CACHE        = 1 << 4,
        const ACCESSED        = 1 << 5,
        const DIRTY           = 1 << 6,
        const HUGE_PAGE       = 1 << 7,
        const GLOBAL          = 1 << 8,
        const NO_EXECUTE      = 1 << 63,
    }
}

impl EntryFlags {
    pub fn from_elf_sections_flag(section: &ElfSection) -> EntryFlags {
        let mut flags = EntryFlags::empty();

        if section.flags().contains(ELF_SECTION_ALLOCATED) {
            // Section is loaded
            flags = flags | PRESENT;
        }
        if section.flags().contains(ELF_SECTION_WRITABLE) {
            // Section is writable
            flags = flags | WRITABLE;
        }
        if !section.flags().contains(ELF_SECTION_EXECUTABLE) {
            // Section is not exectable
            flags = flags | NO_EXECUTE;
        }
        flags
    }
}
