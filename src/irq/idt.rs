
#[repr(C, packed)]
struct Idtr {
    limit: u16,
    offset: u64,
}

struct IdtEntry {
    offset0: u16,
    selector: u16,
    _zero0: u8,
    attr: u8,
    offset1: u16,
    offset2: u32,
    _zero1: u32
}
