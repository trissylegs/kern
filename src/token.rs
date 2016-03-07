
/// Bit field storing unused tokens.
/// 
/// Cannot be constructed: it is passed into the kernel by the boot code.
pub struct Dispenser(u64);

/// Token for Frame allocator. If this value exists then you can assume
/// that all the memory not used by the kernel and multiboot is not used.
/// Consume this value to construct a frame allocator.
pub struct FrameToken;

#[derive(Clone, Copy)]
enum Bit
{
    FrameToken,
}

impl Dispenser
{
    fn take_value(&mut self, value: Bit) -> bool
    {
        if self.0 & (1 << (value as u8)) != 0 {
            self.0 &= !(1 << (value as u8));
            true
        } else {
            false
        }
    }

    pub fn frame_token(&mut self) -> Option<FrameToken>
    {
        if self.take_value(Bit::FrameToken) { Some(FrameToken) } else { None }
    }
}



