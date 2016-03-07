
use super::{Page, ActivePageTable, VirtualAddress};
use super::table::{Table, Level1};
use memory::{Frame, FrameAllocator};

/// Bugs: This only has enough Frames a single mappping.  beacuse
/// ActivePageTable won't release frames used for page tables it
/// once the page is unmapped it won't have enough frames to remap.
pub struct TemporaryPage {
    page: Page,
    allocator: TinyAllocator,
}

impl TemporaryPage {
    pub fn new<A>(page: Page, allocator: &mut A) -> TemporaryPage
        where A: FrameAllocator
    {
        TemporaryPage {
            page: page,
            allocator: TinyAllocator::new(allocator),
        }
    }
    
    pub fn map(&mut self, frame: Frame, active_table: &mut ActivePageTable)
               -> VirtualAddress
    {
        use super::entry::WRITABLE;

        assert!(active_table.translate_page(self.page).is_none(),
                "temporary page is already mapped");
        active_table.map_to(self.page, frame, WRITABLE, &mut self.allocator);
        self.page.start_address()
    }

    pub fn map_table_frame(&mut self,
                           frame: Frame,
                           active_table: &mut ActivePageTable)
                           -> &mut Table<Level1>
    {
        unsafe { &mut *(self.map(frame, active_table) as *mut Table<Level1>) }
    }

    pub fn unmap(&mut self, active_table: &mut ActivePageTable)
    {
        active_table.unmap(self.page, &mut self.allocator)
    }
}

struct TinyAllocator([Option<Frame>; 3]);

impl TinyAllocator {
    fn new(frame_allocator: &mut FrameAllocator) -> TinyAllocator
    {
        TinyAllocator([ frame_allocator.allocate_frame(),
                        frame_allocator.allocate_frame(),
                        frame_allocator.allocate_frame() ])
    }
}

impl FrameAllocator for TinyAllocator
{
    fn allocate_frame(&mut self) -> Option<Frame>
    {
        for slot in &mut self.0 {
            if slot.is_some() {
                return slot.take()
            }
        }
        None
    }

    fn deallocate_frame(&mut self, frame: Frame)
    {
        for slot in &mut self.0 {
            if slot.is_none() {
                *slot = Some(frame);
                return;
            }
        }
        panic!("TinyAllocator can only hold 3 frames");
    }
}
