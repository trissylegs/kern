
pub mod pic;
pub mod isr;

use self::pic::PICS;

pub fn initialize_interrupts()
{
    print!("Initialising PICs... ");
    let mut pics = PICS.lock();
    unsafe {
        pics.initialize();
        println!("Done");
        ::x86::irq::enable();
        // We not in Kansas anymore.
    }
}
