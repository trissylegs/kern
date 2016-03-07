global long_mode_start

extern rust_main
        
section .text
bits 64
;; fn long_mode_start(multiboot_address: usize) -> !
;; Returning from long mode would be bonkers.
long_mode_start:
        ; Assuming multiboot address is in rdi (argument to this function)
        ; Put token::Dispenser in rsi (arg2)
        mov rsi, token.frame_token
        ; Call rust
        call rust_main
        
        ;  Print "OS returned!" (which is bad)
        mov rax, 0x4f724f204f534f4f
        mov [0xb8000], rax
        mov rax, 0x4f724f754f744f65
        mov [0xb8008], rax
        mov rax, 0x4f214f644f654f6e
        mov [0xb8010], rax
        hlt
        
token:
.frame_token: equ (1 << 0)
        
