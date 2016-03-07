
use x86::io::*;
use core::marker::PhantomData;

pub struct UnsafePort<T:PortVal>(u16, PhantomData<T>);
pub struct Port<T:PortVal>(UnsafePort<T>);

impl<T:PortVal> UnsafePort<T>
{
    pub const unsafe fn new(port: u16) -> UnsafePort<T>
    {
        UnsafePort(port, PhantomData)
    }
    pub unsafe fn read(&mut self) -> T
    {
        T::port_in(self.0)
    }
    pub unsafe fn write(&mut self, t: T)
    {
        t.port_out(self.0)
    }
}

impl<T:PortVal> Port<T>
{
    pub const unsafe fn new(port:u16) -> Port<T>
    {
        Port(UnsafePort::new(port))
    }

    pub fn read(&mut self) -> T
    {
        unsafe { self.0.read() }
    }
    pub fn write(&mut self, data: T)
    {
        unsafe { self.0.write(data) }
    }
}

/// Values that can be sent over a port.
/// For most cases u8, u16 and u32 should do.
/// 
pub trait PortVal: Copy {
    /// Read a value out of a port.
    unsafe fn port_in(u16) -> Self;

    /// Write a value to the port.
    unsafe fn port_out(self, u16);
}

/*
/// Currently unused. Might get used if I re-add more strongly typed Ports
///
/// This value is for representing cases where there is no possible
/// error.  Such as port io with type u8. All u8 values are valid u8
/// values. So there is no partialality so we can't get an error.  The
/// debug is so users can just call .unwrap() and the compiler should
/// be able optimise out the unwrap.

pub enum BottomError {}
impl core::fmt::Debug for BottomError {
    fn fmt(&self, &mut core::fmt::Formatter) -> core::fmt::Result {
        // Bottom types are weird.
        match *self {}
    }
}
*/
impl PortVal for u8 {
    unsafe fn port_in(port: u16) -> u8 {
        inb(port)
    }

    unsafe fn port_out(self, port: u16) {
        outb(port, self)
    }
}
impl PortVal for u16 {
    unsafe fn port_in(port: u16) -> u16 {
        inw(port)
    }

    unsafe fn port_out(self, port: u16) {
        outw(port, self)
    }
}
impl PortVal for u32 {
    unsafe fn port_in(port: u16) -> u32 {
        inl(port)
    }

    unsafe fn port_out(self, port: u16) {
        outl(port, self)
    }
}
