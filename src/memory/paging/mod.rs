//! Fun facts
//!
//! Addressing in x86_64
//! There's 2^48 bytes of address space in amd64.
//! * The lowest 12 bits are the index into the page
//! * The next 9 bits are the index in P1 (PT)
//! * The next 9 bits are the index in P2 (PDP)
//! * The next 9 bits are the index in P3 (PD)
//! * The next 9 bits are the index in P4 (PML4)
//! * The remaining bits must be the same as the most signifigant
//! bit in the P4 index. (I.e it's a sign extension)
//!
//! The result of this is that address are split between the high address and low address.
//! The low part 0 -> 2^47-1 and the high part is 2^63 -> 2^64-1
//!
//! I assuming here, but this is probably to prevent programmers from using those
//! bits from something else (GC, Enum's, type info). If they did than x86_64 would be
//! forever doomed with a 48-bit address space because backwards compatibly is something
//! Intel and AMD take very seriously.

pub mod entry;
mod table;
mod temporary_mapping;
mod mapper;

pub use self::entry::*;
pub use self::mapper::Mapper;
use core::ops::{Deref, DerefMut};
use memory::{PAGE_SIZE, Frame, FrameAllocator};
use multiboot2::BootInformation;
use self::temporary_mapping::TemporaryPage;
use x86::controlregs;
use x86::tlb;


/// Number of entries in each page tables.
const ENTRY_COUNT: usize = 512;

pub type PhysicalAddress = usize;
pub type VirtualAddress = usize;

#[derive(Debug, Clone, Copy)]
pub struct Page {
    number: usize,
}

impl Page
{
    pub fn containing_address(address: VirtualAddress) -> Page
    {
        assert!(address <  0x0000_8000_0000_0000 ||
                address >= 0xffff_8000_0000_0000,
                "invalid address: {:#x}", address);
        Page { number: address / PAGE_SIZE }
    }

    pub fn start_address(&self) -> usize
    {
        self.number * PAGE_SIZE
    }

    fn p4_index(&self) -> usize { (self.number >> 27) & 0o777 }
    fn p3_index(&self) -> usize { (self.number >> 18) & 0o777 }
    fn p2_index(&self) -> usize { (self.number >>  9) & 0o777 }
    fn p1_index(&self) -> usize { (self.number >>  0) & 0o777 }
}


pub fn test_paging<A>(allocator: &mut A)
    where A: FrameAllocator
{
    let mut page_table = unsafe { ActivePageTable::new() };

    // 1st page table entry
    println!("Some = {:?}", page_table.translate(0));
    // 2nd page table entry
    println!("Some = {:?}", page_table.translate(4096));
    // 2nd P2 entry
    println!("Some = {:?}", page_table.translate(512 * 4096));
    // 300th p2 entry
    println!("Some = {:?}", page_table.translate(300 * 512 * 4096));
    // 3rd p3 entry (should be none)
    println!("None = {:?}", page_table.translate(512 * 512 * 4096));
    // Last allocated byte (used)
    println!("Some = {:?}", page_table.translate(512 * 512 * 4096 - 1));

    println!("");
    
    // 42 p3 entry
    let addr = 42 * 512 * 512 * 4096;
    let page = Page::containing_address(addr);
    let frame = allocator.allocate_frame().expect("out of memory");
    println!("None = {:?}, map to {:?}",
             page_table.translate(addr),
             frame);
    page_table.map_to(page, frame, EntryFlags::empty(), allocator);
    println!("Some = {:?}", page_table.translate(addr));
    println!("next free frame: {:?}", allocator.allocate_frame());

    println!("{:#x}", unsafe {
        *(Page::containing_address(addr).start_address() as *const u64)
    });
    
    page_table.unmap(Page::containing_address(addr), allocator);
    println!("None = {:?}", page_table.translate(addr));

    // println!("{:#x}", unsafe {
    //     *(Page::containing_address(addr).start_address() as *const u64)
    // });
}

pub struct ActivePageTable {
    mapper: Mapper,
}

impl Deref for ActivePageTable {
    type Target = Mapper;
    fn deref(&self) -> &Mapper {
        &self.mapper
    }
}

impl DerefMut for ActivePageTable {
    fn deref_mut(&mut self) -> &mut Mapper {
        &mut self.mapper
    }
}

impl ActivePageTable {
    unsafe fn new() -> ActivePageTable
    {
        ActivePageTable {
            mapper: Mapper::new(),
        }
    }

    pub fn translate(&self, address: VirtualAddress) -> Option<PhysicalAddress>
    {
        self.mapper.translate(address)
    }

    pub fn switch(&mut self,
                new_table: InactivePageTable)
                -> InactivePageTable
    {
        let old_table = InactivePageTable {
            p4_frame: Frame::containing_address(unsafe { controlregs::cr3() }
                                                as usize),
        };
        unsafe {
            controlregs::cr3_write(new_table.p4_frame.start_address() as u64);
        }
        // Hopefully we don't crash.
        old_table
            
    }
    
    pub fn with<F>(&mut self,
                   table: &mut InactivePageTable,
                   temporary_page: &mut TemporaryPage,
                   f: F)
        where F: FnOnce(&mut Mapper)
    {
        {
            let flush_tlb = || unsafe { tlb::flush_all() };
        
            let backup = Frame::containing_address(unsafe {
                // Unsafe because this causes a exception in Ring 3, but
                // this is only called in Ring 0, so we're safe.
                controlregs::cr3() as usize
            });

            // map temporary_page to current p4 table
            let p4_table = temporary_page.map_table_frame(backup.clone(), self);

            // overwrite recursive mapping
            self.p4_mut()[511].set(table.p4_frame.clone(), PRESENT | WRITABLE);
            flush_tlb();

            // Execute the callback with the new context.
            f(self);

            // restore recursive mapping
            p4_table[511].set(backup, PRESENT | WRITABLE);
            flush_tlb();
        }
        temporary_page.unmap(self);
    }
}

pub struct InactivePageTable {
    p4_frame: Frame,
}

impl InactivePageTable {
    pub fn new(frame: Frame,
               active_table: &mut ActivePageTable,
               temporary_page: &mut TemporaryPage)
               -> InactivePageTable
    {
        {
            let table = temporary_page.map_table_frame(frame.clone(),
                                                     active_table);
            table.zero();
            table[511].set(frame.clone(), PRESENT | WRITABLE);
        }
        temporary_page.unmap(active_table);

        InactivePageTable { p4_frame: frame }
    }
}

/// Recreate the page table such that only the kernel, vga buffer, and
/// multiboot structures are in memory. (And that they're properly
/// protected).
pub fn remap_the_kernel<A>(allocator: &mut A,
                           multiboot_pos: (usize, usize),
                           boot_info: &BootInformation)
                           -> ActivePageTable
    where A: FrameAllocator
{
    use core::ops::Range;

    println!("Remapping the kernel");
    
    // 0xcafebabe is a arbitary address
    let mut temporary_page = TemporaryPage::new(Page { number: 0xcafebabe },
                                                allocator);

    let mut active_table = unsafe { ActivePageTable::new() };
    let mut new_table = {
        let frame = allocator.allocate_frame().expect("no more frame");
        InactivePageTable::new(frame, &mut active_table, &mut temporary_page)
    };

    println!("Switch recursive mapping");
    active_table.with(&mut new_table, &mut temporary_page, |mapper| {
        let elf_sections_tag = boot_info.elf_sections_tag()
            .expect("Memory map tag required");

        println!("Remapping elf sections.");
        for section in elf_sections_tag.sections() {
            use multiboot2::ELF_SECTION_ALLOCATED;

            if !section.flags().contains(ELF_SECTION_ALLOCATED) {
                // Section not loaded
                continue;
            }

            println!("mapping section at addr: {:#x}, size: {:#x}",
                     section.addr, section.size);

            // TODO use the real section tags
            let flags = EntryFlags::from_elf_sections_flag(section);

            let range = Range {
                start: section.addr as usize,
                end:   (section.addr + section.size) as usize,
            };
            
            for address in range.step_by(PAGE_SIZE) {
                assert!(address % PAGE_SIZE == 0,
                        "sections need to be page aligned");
                let frame = Frame::containing_address(address);
                mapper.identity_map(frame, flags, allocator);
            }
        }

        println!("Remapping VGA buffer");
        // Identity map the VGA text buffer.
        let vga_buffer_frame = Frame::containing_address(0xb8000);
        mapper.identity_map(vga_buffer_frame, WRITABLE | NO_EXECUTE, allocator);

        // Remapping multiboot.
        println!("Remapping Multiboot");
        let range = (multiboot_pos.0)..(multiboot_pos.1);
        for address in range.step_by(PAGE_SIZE) {
            let frame = Frame::containing_address(address);
            mapper.identity_map(frame, NO_EXECUTE, allocator);
        }
    });

    let old_table = active_table.switch(new_table);
    println!("NEW TABLE!!!");

    // Change the old p4 page into a guard page.
    let old_p4_page = Page::containing_address(old_table.p4_frame.start_address());
    active_table.unmap(old_p4_page, allocator);
    println!("guard page at {:#x}", old_p4_page.start_address());

    active_table
}
