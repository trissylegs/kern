
use memory::{Frame, FrameAllocator};
use multiboot2::{MemoryAreaIter, MemoryArea};
use token::FrameToken;

/// Pulls memory from these areas.
/// 
/// * 0 -> vga buffer
/// * vga buffer -> multiboot info
/// * multiboot info -> kernel
/// * kernel -> 1st hole
/// * 1st hole -> end of ram | 2nd hole
/// * [ 2nd hole -> end of ram ]
pub struct AreaFrameAllocator {
    next_free_frame: Frame,
    current_area: Option<&'static MemoryArea>,
    areas: MemoryAreaIter,
    // Kernel start and end.
    kernel_start: Frame,
    kernel_end:  Frame,
    // Multiboot start and end.
    multiboot_start: Frame,
    multiboot_end: Frame,

    // Counter is for profiling and destruction.
    alloc_count: usize,
    dealloc_count: usize,
}

impl AreaFrameAllocator {
    /// Consume the FrameToken and take over allocation of unused frames.
    ///
    /// It cannot restore the frame token. As there's no guarentee
    /// that frames returned to it were originally from it. (You can
    /// return dicarded kernel frames which is not represented by the
    /// FrameToken)
    pub fn new(_token: FrameToken,
               kernel_start: usize,    kernel_end: usize,
               multiboot_start: usize, multiboot_end: usize,
               memory_areas: MemoryAreaIter)
               -> AreaFrameAllocator
    {
        let mut allocator = AreaFrameAllocator {
            next_free_frame: Frame::containing_address(0),
            current_area: None,
            areas: memory_areas,
            kernel_start: Frame::containing_address(kernel_start),
            kernel_end: Frame::containing_address(kernel_end),
            multiboot_start: Frame::containing_address(multiboot_start),
            multiboot_end: Frame::containing_address(multiboot_end),
            alloc_count: 0,
            dealloc_count: 0,
        };
        allocator.choose_next_area();
        allocator
    }
    
    fn choose_next_area(&mut self) {
        self.current_area = self.areas.clone().filter(|area| {
            let address = area.base_addr + area.length - 1;
            Frame::containing_address(address as usize) >= self.next_free_frame
        }).min_by_key(|area| area.base_addr);
        
        if let Some(area) = self.current_area {
            let start_frame = Frame::containing_address(area.base_addr as usize);
            if self.next_free_frame < start_frame {
                self.next_free_frame = start_frame;
            }
        }
    }

    pub fn get_alloc_counts(&self) -> (usize, usize)
    {
        (self.alloc_count, self.dealloc_count)
    }
}

impl FrameAllocator for AreaFrameAllocator {
    fn allocate_frame(&mut self) -> Option<Frame> {
        // The original uses recursion, but we have VERY little stack. Because rust does
        // not guarentee tail call elimination we use a loop instead.
        while let Some(area) = self.current_area {
            let frame = self.next_free_frame.clone();
            
            // The last frame of the current area
            let current_area_last_frame = {
                let address = area.base_addr + area.length - 1;
                Frame::containing_address(address as usize)
            };

            if frame > current_area_last_frame {
                // We've used up one area. Go on to next one.
                self.choose_next_area();
            } else if frame >= self.kernel_start && frame <= self.kernel_end {
                // Frame is used by the kernel
                self.next_free_frame = Frame {
                    number: self.kernel_end.number + 1
                };
            } else if frame >= self.multiboot_start && frame <= self.multiboot_end {
                // Frame is used by multiboot
                self.next_free_frame = Frame {
                    number: self.multiboot_end.number + 1
                }
            } else {
                self.next_free_frame.number += 1;
                self.alloc_count += 1;
                return Some(frame);
            }
        }
        None
    }
       
    fn deallocate_frame(&mut self, _frame: Frame)
    {
        self.dealloc_count += 1;
        // KTHXBAI
    }
}
