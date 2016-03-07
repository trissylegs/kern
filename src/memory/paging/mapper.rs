//

use super::{VirtualAddress, PhysicalAddress, Page, ENTRY_COUNT};
use super::entry::*;
use super::table::{self, Table, Level4};
use memory::{Frame, FrameAllocator};
use core::ptr::Unique;

pub struct Mapper {
    p4: Unique<Table<Level4>>,
}

impl Mapper
{
    pub unsafe fn new() -> Mapper
    {
        Mapper {
            p4: Unique::new(table::P4),
        }
    }

    pub fn p4(&self) -> &Table<Level4>
    {
        unsafe { self.p4.get() }
    }

    pub fn p4_mut(&mut self) -> &mut Table<Level4>
    {
        unsafe { self.p4.get_mut() }
    }

    pub fn translate(&self, virtual_address: VirtualAddress) -> Option<PhysicalAddress>
    {
        let page = Page::containing_address(virtual_address);
        self.translate_page(page).map(|frame| frame.start_address())
    }
    
    pub fn translate_page(&self, page: Page) -> Option<Frame>
    {
        let p3 = self.p4().next_table(page.p4_index());
        
        // Yep, the huge page code is longer than the rest of function
        // I might be possible to cut it in half.
        let huge_page = || {
            p3.and_then(|p3| {
                let p3_entry = &p3[page.p3_index()];
                // 1GiB page
                if let Some(start_frame) = p3_entry.pointed_frame() {
                    if p3_entry.flags().contains(HUGE_PAGE) {
                        assert!(start_frame.number % (ENTRY_COUNT * ENTRY_COUNT) == 0,
                                "1GiB huge page wasn't aligned");
                        return Some(Frame {
                            number: start_frame.number + page.p2_index() * ENTRY_COUNT +
                            page.p1_index(),
                        });
                    }
                }
                if let Some(p2) = p3.next_table(page.p3_index()) {
                    let p2_entry = &p2[page.p2_index()];
                    // 2MiB page
                    if let Some(start_frame) = p2_entry.pointed_frame() {
                        if p2_entry.flags().contains(HUGE_PAGE) {
                            assert!(start_frame.number % ENTRY_COUNT == 0,
                                    "2MiB huge page wasn't aligned");
                            return Some(Frame {
                                number: start_frame.number + page.p1_index()
                            });
                        }
                    }
                }
                None
            })
        };
        
        p3.and_then(|p3| p3.next_table(page.p3_index()))
            .and_then(|p2| p2.next_table(page.p2_index()))
            .and_then(|p1| p1[page.p1_index()].pointed_frame())
            .or_else(huge_page)
    }

    pub fn map_to<A>(&mut self,
                     page: Page,
                     frame: Frame,
                     flags: EntryFlags,
                     allocator: &mut A)
        where A: FrameAllocator
    {
        let mut p4 = self.p4_mut();
        let mut p3 = p4.next_table_create(page.p4_index(), allocator);
        let mut p2 = p3.next_table_create(page.p3_index(), allocator);
        let mut p1 = p2.next_table_create(page.p2_index(), allocator);
        
        assert!(p1[page.p1_index()].is_unused());
        p1[page.p1_index()].set(frame, flags | PRESENT);
    }

    pub fn map<A>(&mut self, page: Page, flags: EntryFlags, allocator: &mut A)
        where A: FrameAllocator
    {
        let frame = allocator.allocate_frame().expect("out of memory");
        self.map_to(page, frame, flags, allocator);
    }

    pub fn unmap<A>(&mut self, page: Page, allocator: &mut A)
        where A: FrameAllocator
    {
        assert!(self.translate(page.start_address()).is_some());
        let p1 = self.p4_mut()
            .next_table_mut(page.p4_index())
            .and_then(|p3| p3.next_table_mut(page.p3_index()))
            .and_then(|p2| p2.next_table_mut(page.p2_index()))
            .expect("page lookup failed, or contained huge page");
        let frame = p1[page.p1_index()].pointed_frame().unwrap();
        p1[page.p1_index()].set_unused();

        // TODO: Flush TLB
        unsafe {
            ::x86::tlb::flush(page.start_address())
        }
        
        // Todo: free frames used by empty page tables
        allocator.deallocate_frame(frame);
    }

    /// Identity map the the given frame with the provided flags.
    /// The `FrameAllocator` is used to create new page tables if needed.
    pub fn identity_map<A>(&mut self,
                           frame: Frame,
                           flags: EntryFlags,
                           allocator: &mut A)
        where A: FrameAllocator
    {
        let addr: VirtualAddress = frame.start_address();
        self.map_to(Page::containing_address(addr), frame, flags, allocator);
    }

}
