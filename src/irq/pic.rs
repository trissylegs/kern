
use spin::Mutex;
use port::{Port, UnsafePort};

pub struct Pic {
    offset: u8,
    command: UnsafePort<u8>,
    data: UnsafePort<u8>,
}

#[repr(u8)]
#[derive(Clone, Copy)]
enum Icw1 {
    Icw4Needed = 0x01,          // ICW4 needed
    Single     = 0x02,          // Single mode
    Interval4  = 0x04,          // Call address interval 4
    Level      = 0x08,          // Level triggered
    InitBit    = 0x10,          // This is an initialisation command
}

#[repr(u8)]
#[derive(Clone, Copy)]
enum Icw4 {
    Mode8086  = 0x01,           // 8086/88 (MCS-80/85) mode
    Auto      = 0x02,           // Auto EOI
    BufSlave  = 0x08,           // Buffered mode (slave)
    BufMaster = 0x0c,           // Buffered mode (master)
    SFNM      = 0x10,           // Special fully nested
}

#[repr(u8)]
#[derive(Clone, Copy)]
enum Cmd {
    Init = (Icw1::Icw4Needed as u8) | (Icw1::InitBit as u8),
    EndOfInterrupt = 0x20,
    ReadIRR = 0x0a,
    ReadISR = 0x0b,
}

impl Pic {
    fn handles_interrupt(&self,
                         interrupt_id: u8)
                         -> bool
    {
        self.offset <= interrupt_id && interrupt_id < self.offset + 8
    }

    unsafe fn end_of_interrupt(&mut self)
    {
        self.command.write(Cmd::EndOfInterrupt as u8);
    }
}

pub struct ChainedPics {
    initialized: bool,
    pics: [Pic; 2],
}

/// The chained 8259 PICs. They are set to Interrupt vectors 0x20 and 0x28.
/// They need to be at least 0x20 because Intel also uses 0x0..0x1f for
/// exceptions.
///
/// This is not initialized until .initalise is called.
/// (Obviously can't do that statically)
pub static PICS: Mutex<ChainedPics> = Mutex::new(unsafe { ChainedPics::new(0x20, 0x28) });

impl ChainedPics {
    const unsafe fn new(offset1: u8,
                        offset2: u8)
                        -> ChainedPics
    {
        ChainedPics {
            initialized: false,
            pics: [
                Pic {
                    offset: offset1,
                    command: UnsafePort::new(0x20),
                    data: UnsafePort::new(0x21),
                },
                Pic {
                    offset: offset2,
                    command: UnsafePort::new(0xA0),
                    data: UnsafePort::new(0xA1),
                }]
        }
    }

    pub unsafe fn initialize(&mut self)
    {
        let saved_mask1 = self.pics[0].data.read();
        let saved_mask2 = self.pics[1].data.read();

        // ICW1
        // Start initialisation sequence
        self.pics[0].command.write(Cmd::Init as u8);
        io_wait();
        self.pics[1].command.write(Cmd::Init as u8);
        io_wait();

        // ICW2
        // Set vector offset
        self.pics[0].data.write(self.pics[0].offset);
        io_wait();
        self.pics[1].data.write(self.pics[1].offset);
        io_wait();

        // ICW3
        // Inform pic1 that it has a slave at irq2
        self.pics[0].data.write(4);
        io_wait();
        // Tell pic2 it's cascaded identity
        self.pics[1].data.write(2);
        io_wait();

        // ICW4
        // Use 8086/88 mode
        self.pics[0].data.write(Icw4::Mode8086 as u8);
        io_wait();
        self.pics[1].data.write(Icw4::Mode8086 as u8);
        io_wait();
        
        self.pics[0].data.write(saved_mask1);
        self.pics[1].data.write(saved_mask2);
        
        self.initialized = true;
    }

    fn check(&self)
    {
        assert!(self.initialized, "PIC used before initialisation");
    }
    
    pub fn handles_interrupt(&self, interrupt_id: u8) -> bool
    {
        self.pics.iter().any(|pic| pic.handles_interrupt(interrupt_id))
    }

    pub unsafe fn end_of_interrupt(&mut self, interrupt_id: u8)
    {
        self.check();
        assert!(self.handles_interrupt(interrupt_id),
                "tried to EOI an unknown interrupt_id");
        if self.pics[1].handles_interrupt(interrupt_id) {
            self.pics[1].end_of_interrupt()
        }
        self.pics[0].end_of_interrupt();
    }

    unsafe fn select_irq(&mut self, irq_line: u8) -> (&mut UnsafePort<u8>, u8)
    {
        if irq_line < 8 {
            (&mut self.pics[0].data, irq_line)
        } else {
            (&mut self.pics[1].data, irq_line - 8)
        }
    }

    pub unsafe fn set_mask(&mut self, irq_line: u8)
    {
        self.check();
        let (port, irq_line) = self.select_irq(irq_line);
        let mask = port.read();
        port.write(mask | (1 << irq_line))
    }

    pub unsafe fn clear_mask(&mut self, irq_line: u8)
    {
        self.check();
        let (port, irq_line) = self.select_irq(irq_line);
        let mask = port.read();
        port.write(mask & !(1 << irq_line))
    }

    /// Get the requested register for both PICs.
    unsafe fn read_irq_reg(&mut self, reg: Cmd) -> (u8, u8)
    {

        self.pics[0].command.write(reg as u8);
        self.pics[1].command.write(reg as u8);
        (self.pics[0].command.read(),
         self.pics[1].command.read())
    }

    pub fn read_irr(&mut self) -> (u8, u8)
    {
        self.check();
        unsafe { self.read_irq_reg(Cmd::ReadIRR) }
    }

    pub fn read_isr(&mut self) -> (u8, u8)
    {
        self.check();
        unsafe { self.read_irq_reg(Cmd::ReadISR) }
    }
}

/// Do a short wait. Long enough for configuring PICs.
/// Works by writing to an unused port.
fn io_wait()
{
    // 0x80 is used during POST but does nothing after boot.  But
    // writing to it consumes time so we just write 0 to it to wait.
    // This technique is used by kexec for Linux.  Which is possibly
    // enough evidence that it works.
    let mut wait_port: Port<u8> = unsafe { Port::new(0x80) };
    wait_port.write(0);
}
