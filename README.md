
# Kern

A x86_64 kernel based on Phillipp Oppermann's blog series.
http://os.phil-opp.com/

## DIY

Patch libcore. You need to patch libcore with the patch found in
`./libcore`
and compile it with x86_64-unknown-none-gnu and install that.
This is so we can disable floating point with
`--cfg disable_float`

One way you might do this is (assuming you're using multirust)

```sh
target=x86_64-unknown-none-gnu
  
cd libcore
wget $rustc_nightly_url
tar -xvf rustc-nightly-src
cp rustc-nightly-src/src/libcore libcore
patch -p0 < libcore_nofp.patch
cd ..
target=x86_64-unknown-none-gnu
rustc --target $(target) -Z no-landing-pads                                 \
      --cfg disable-float                                                   \
      --out-dir ~/.multirust/toolchains/nightly/lib/rustlib/$(target)/lib   \
      libcore/libcore/lib.rs
```

## Current features

* Rust is started in long mode.
* print! & println! print to the VGA buffer.
* panic! will print the panic message in bright red with file and line number.
* Multiboot memory and elf tags are read.
* Frame allocation. (no deallocation)
* The current page table is paged into the last 512 GiB of address space
using recursive mapping.
(There is still ~255 TiB of address space left so don't panic.)
* Memory below 1GiB (0x40000000) is mapped 1 to 1 to its hardward address.
(Identity mapping)

## Planned features

* Interrupts
* Ports
* Frame deallocation.
* Page allocation/dealloction (currently is BYO pages)
* Make the kernel mapping read-only. 
* Remove the identity mapping


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
disables floating pointer registers. The "red zone" so that interrupts
can be handled safely and efficiently.

It was written by stolen from:
http://www.randomhacks.net/2015/11/11/bare-metal-rust-custom-target-kernel-space/
on 2016-01-08.  So thanks to Eric Kidd for that file.
