
# Kern

A x86_64 kernel based on Phillipp Oppermann's blog series.
http://os.phil-opp.com/

## DIY

Due to the fact that saving SIMD registers takes a FXSAVE instruction we disable
SIMD and floating point (Sys-V abi needs SIMD for float args/return)
you will need to specially compile a patched verion of libcore for
x86-unknown-none-gnu.
 (See bellow for a link)

## Bugs:

* Triple faults into a reboot loop (current bug)

## Current features

* Rust is started in long mode.
* print! & println! print to the VGA buffer.
* panic! will print the panic message in bright red with file and line number.
* Multiboot memory and elf tags are read.
* Frame allocation. (no deallocation)
* The current page table is paged into the last 512 GiB of address space
using recursive mapping.
* The kernel and Mutliboot data is mapped appropriately. (Mulitboot is
read only and Kernel is according to ELF info)

## Planned features

* Interrupts
* Ports
* Frame deallocation.
* Page allocation/dealloction (currently is pick your own pages)

## In the long term

* Useful things an OS should have

## Boot Errors / Requirements

Currently we fail and halt on boot when:

* `ERR: 0`, multiboot was not used to start the kernel.
* `ERR: 1`, CPUID is not available
* `ERR: 2`, long mode is not supported. (x86_64 mode)
* `ERR: a`, SSE is not supported. (May be disable in future)

(The reason these errors are so short is that they need to be loaded
in assembly and keeping the assembly short is easier.

## x86_64-unknown-none-gnu

The ABI is defined in `x86_64-unknown-none-gnu.json`. This files
disables floating pointer registers and the "red zone" so that interrupts
can be handled safely and efficiently.

It was stolen from:
http://www.randomhacks.net/2015/11/11/bare-metal-rust-custom-target-kernel-space/
on 2016-01-08.  So thanks to Eric Kidd for that file.

(This also gives info on how to recompile libcore)


