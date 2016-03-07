
#[repr(C, packed)]
struct Gdt {
    /// bits 0:15 of limit
    limit0: u16,
    /// bits 0:15 of base
    base0: u16,
    /// bits 16:23 of base
    base1: u8,
    /// access bits
    access: u8,
    /// 
    flags_limits: u8
}
