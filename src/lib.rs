// Core kernel code.

#![feature(lang_items)]
#![feature(const_fn)]
#![feature(unique)]
#![feature(step_by)]
#![feature(asm)]
#![feature(core_intrinsics)]
#![no_std]

// Lots of dead code until we acutally start using it.
// So this temporily here so I can see the useful warning.
#![allow(dead_code)]

extern crate rlibc;
// extern crate spin;
extern crate multiboot2;
#[macro_use]
extern crate bitflags;
extern crate x86;

#[macro_use]
pub mod vga;
pub mod memory;
pub mod port;
pub mod irq;
pub mod token;
pub mod spin;

// CAUTION: We have a small stack and no guard page.  Go too far
// and we rewrite the page table.  I guess that will cause a PageFault
// anyway.

#[no_mangle]
pub extern fn rust_main(multiboot_information_address: usize,
                        dispenser: token::Dispenser)
{
    let mut dispenser = dispenser;
    println!("Kernel started!");
    println!("");

    // println!("multiboot: {:x}", multiboot_information_address);
    // halt();

    println!("Enabling NXE bit");
    enable_nxe_bit();

    println!("Enabling WP bit");
    enable_write_protect_bit();
    
    let boot_info = unsafe { multiboot2::load(multiboot_information_address) };
    let memory_map_tag = boot_info.memory_map_tag()
        .expect("Memory map tag required");

    println!("memory areas:");
    for area in memory_map_tag.memory_areas() {
        println!("   start: {:#16x}, length: {:#16x}", area.base_addr, area.length);
    }

    let elf_sections_tag = boot_info.elf_sections_tag()
        .expect("Elf-section tag required");

    println!("kernel sections:");
    for section in elf_sections_tag.sections() {
        println!("    addr: {:#16x}, length: {:#16x}, flags: {:#16x}",
                 section.addr, section.size, section.flags);
    }

    let kernel_start = elf_sections_tag.sections().map(|s| s.addr)
        .min().unwrap();
    let kernel_end = elf_sections_tag.sections().map(|s| s.addr + s.size)
        .max().unwrap();

    let multiboot_start = multiboot_information_address;
    let multiboot_end = multiboot_start + (boot_info.total_size as usize);

    println!("kernel_start:    {:#8x}, end: {:#8x}", kernel_start, kernel_end);
    println!("multiboot_start: {:#8x}, end: {:#8x}", multiboot_start, multiboot_end);

    let frame_token = dispenser.frame_token().expect("Frame token missing");
    let mut frame_allocator =
        memory::AreaFrameAllocator::new(frame_token,
                                        kernel_start as usize, kernel_end as usize,
                                        multiboot_start, multiboot_end,
                                        memory_map_tag.memory_areas());

    // this is the new part
    let page_table = memory::paging::remap_the_kernel(&mut frame_allocator,
                                                      (multiboot_start, multiboot_end),
                                                      boot_info);
    println!("It did not crash!");

    let (alloc, dealloc) = frame_allocator.get_alloc_counts();
    println!("Allocated {} frames.", alloc);
    println!("Deallocated {} frames.", dealloc);

    print!("Checking multiboot is still in memory... ");
    if page_table.translate(multiboot_information_address).is_none() {
        panic!("Multiboot no longer mapped");
    } else {
        println!("All good");
    }

    println!("Initialising interrupts");
    irq::initialize_interrupts();
    halt();
}

/// Halt the processor with the hlt instruction.
/// If unavilibe will infinite loop.
/// Generally this function will never return.
fn halt() -> !
{
    unsafe { asm!("hlt"); }
    loop {}
}

fn enable_nxe_bit()
{
    use x86::msr::{IA32_EFER, rdmsr, wrmsr};
    let nxe_bit = 1 << 11;
    unsafe {
        let efer = rdmsr(IA32_EFER);
        wrmsr(IA32_EFER, efer | nxe_bit);
    }
}

fn enable_write_protect_bit()
{
    use x86::controlregs::{cr0, cr0_write};
    let wp_bit = 1 << 16;
    unsafe { cr0_write(cr0() | wp_bit) };
}

#[lang = "eh_personality"]
extern fn eh_personality()
{
}

#[lang = "panic_fmt"]
extern fn panic_fmt(fmt: core::fmt::Arguments,
                    file: &str,
                    line: u32) -> !
{
    use vga::*;
    use core::fmt::Write;

    let mut lock = vga::WRITER.lock();
    
    lock.set_color(ColorCode::new(Color::LightRed, Color::Black));
    lock.write_bytes(&[b'*'; 80][..]);
    lock.write_fmt(format_args!("\nPANIC in {} at line {}:", file, line)).ok();
    lock.write_fmt(format_args!("\t{}", fmt)).ok();
    lock.write_bytes(&[b'*'; 80][..]);
    
    halt();
}

