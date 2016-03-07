
use memory::paging::VirtualAddress;
use core::intrinsics::{volatile_store, volatile_load};
use core::u64;

// 0 is a valid interrupt so MAX is used to signify a non
static mut LAST_INTERRUPT: u64 = u64::MAX;

static EXCEPTION_NAME: [&'static str; 21] =
    [ "Divide Error",
      "Debug Exeception",
      "NMI Interrupt (not an exception)",
      "Breakpoint",
      "Overflow",
      "BOUND Range exceeded",
      "Invalid Opcode (Undefined Opcode)",
      "Device not Available (No Math Coprocessor)",
      "Double Fault",
      "Coprocessor Segment overrun",
      "Invalid TSS",
      "Segment not present",
      "Stack-Segment fault",
      "General Protection fault",
      "Page fault",
      "__RESERVED__",
      "x87 FPU Floating-Point Error (Math fault)",
      "Alignment check",
      "Machine Check",
      "SIMD Floating-Point exception",
      "Virtualisation Exception", ];

#[no_mangle]
pub extern fn interrupt_handler(number: u64, _stack_address: VirtualAddress)
{
    unsafe {
        volatile_store(&mut LAST_INTERRUPT, number);
    }
    // Shouldn't return from these interrupts so panic instead.
    match number {
        0 | 1 | 3...21 => panic!("CPU Exception ({}): {}", number,
                                 EXCEPTION_NAME[number as usize]),
        22...31 => panic!("Intel reserved interrupt: {}", number),
        _ => (),
    }
}

pub fn get_last_interrupt() -> Option<u64>
{
    unsafe {
        match volatile_load(&LAST_INTERRUPT) {
            x if x <= 0xff => Some(x),
            _ => None,
        }
    }
}
