;; -*- mode: nasm-mode; -*-

global load_idtr
extern interrupt_handler

bits 64
isr:
.entry:
        dq 0
.handle:
        push rbp
        push r15
        push r14
        push r13
        push r12
        push r11
        push r10
        push r9
        push r8
        push rsi
        push rdi
        push rdx
        push rcx
        push rbx
        push rax

        ; Argumements
        mov rdi, qword [.entry] ; interrupt_number: u8,
        mov rsi, rsp            ; stack_address: u64

        ; SYS-V requires 16 byte alignment of stack pointer
        ; This will round it downwards to the nearest multiple of 16.
        mov rbp, rsp
        and rsp, 0xfffffffffffffff0
        
        call interrupt_handler

        push rax
        push rbx
        push rcx
        push rdx
        push rdi
        push rsi
        push r8
        push r9
        push r10
        push r11
        push r12
        push r13
        push r14
        push r15
        push rbp
        
        iretq
        
;; Put the interrupt number into the address .entry
;; then jump to .handle
.int0:
        mov [.entry], byte 0
        jmp qword .handle
.int1:
%assign i 1
%rep 255        
        mov [.entry], byte i
        jmp qword .handle
%assign i (i+1)
%endrep        

;; Load the idt register.
load_idtr:
        lidt [idtr]
        ret
        
idtr:
        dw (idt.end - idt) + 1
        dq idt
        
idt:
%assign i 0
%rep 256
        dw isr.int0 - (isr.int1 - isr.int0) * i ; offset
        dw gdt64.code                           ; gdt section
        db 0                                    ; 
        db (0b1110) | (1 << 7)                  ; interrupt64 | present
        dw 0
        dd 0                    ; reserved
        dd 0
%assign i (i+1)
%endrep
.end:
        
