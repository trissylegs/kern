
extern long_mode_start
extern idtr
        
global start
global gdt.kernel_code        

section .text
bits 32
start:
        mov  esp, stack_top     ; Set up stack pointer
        mov  edi, ebx           ; Save multiboot address
        call check_multiboot
        call check_cpuid
        call check_long_mode

        call set_up_page_tables
        call enable_paging
        call set_up_SSE

        ;; Load the 64-bit GDT
        lgdt [gdtr]

        ; Load the idt register.
        ; lidt [idtr]

        ;; Load the selectors
        mov ax, gdt.kernel_data
        mov ss, ax              ; stack selector
        mov ds, ax              ; data selector
        mov es, ax              ; extra selector

        jmp gdt.kernel_code:long_mode_start

        ; Oh dear
        hlt

;; Function: error
;; Prints 'ERR: ' and the given error code to the screen and hangs.
;; param: al, error code as an ascii value
error:
        mov dword [0xb8000], 0x4f524f45
        mov dword [0xb8004], 0x4f3a4f52
        mov dword [0xb8008], 0x4f204f20
        mov byte  [0xb800a], al
        hlt

;; Function: check_multiboot
;; Check that we were called by multiboot
check_multiboot:
        cmp eax, 0x36d76289
        jne .no_multiboot
        ret
.no_multiboot:
        mov al, "0"
        jmp error

;; Function: check_cpuid
;; Check if CPUID is supported by attempting to flip the ID bit (21) in
;; the FLAGS register. If we can flip it, CPUID is available.
check_cpuid:
        ;; Copy flags in to EAX via stack
        pushfd
        pop eax

        ;; For comparison later on
        mov ecx, eax

        ;; Flip the id bit
        xor eax, 1 << 21

        ;; Copy EAX to flags via stack
        push eax
        popfd

        ;; Copy FLAGS back to EAX
        pushfd
        pop eax

        ;; Restore FLAGS
        push ecx
        popfd

        ;; Compare to check if bit 21 was succesfully changed
        xor eax, ecx
        jz .no_cpuid
        ret
.no_cpuid:
        mov al, "1"
        jmp error

;; Function: check_long_mode
check_long_mode:
        mov eax, 0x80000000     ; Used to check or long mode later
        cpuid                   ; Get the cpu identification
        cmp eax, 0x80000001     ; If CPUID is less that this. Then
        jb .no_long_mode        ; long mode is not supported
        mov eax, 0x80000001     ;
        cpuid
        test edx, 1 << 29       ; Test if the LM-bit is set in the D-register
        jz .no_long_mode        ; There is no long mode
        ret
.no_long_mode:
        mov al, "2"
        jmp error

;; Function: set_up_page_tables
;; Set's up the first GiB of Memory with identity paging.
;; (Virtual RAM maps to physical ram)
;; Also add recursive mapping to the last entry of the p4 table
set_up_page_tables:
        mov eax, p4_table       ; Recursively map p4 to itself in it's last entry.
        or  eax, 0b11           ; Present + writable
        mov [p4_table + 511 * 8] , eax
        
        ;; Map first p4 entry to p3 
        mov eax, p3_table
        or  eax, 0b11           ; Present + writable
        mov [p4_table], eax

        ;; Map first p3 entry to p2
        mov eax, p2_table
        or  eax, 0b11            ; Present + writable
        mov [p3_table], eax

        ;; Map each P2 eantry to a huge 2MiB page
        mov ecx, 0              ; counter

.map_p2_table:
        ;; map ecx-th P2 entry to a huge page that starts at address 2MiB*ecx
        mov eax, 0x200000       ; 2 MiB
        mul ecx                 ; start address of ecx-th page
        or eax, 0b10000011      ; present + writable + huge
        mov [p2_table + ecx * 8], eax ; map the ecx-th entry

        inc ecx                 ; increase counter
        cmp ecx, 512            ; if counter == 512, all entries have been written
        jne .map_p2_table

        ret

enable_paging:
        ;; load P4 to CR3 register (CPU looks for Page Table here)
        mov eax, p4_table
        mov cr3, eax

        ;; enable PAE-flag in CR4 (Physical Address Extension)
        mov eax, cr4
        or  eax, 1 << 5
        mov cr4, eax

        ;; set the long most bit in the EFER MSR (model specific register)
        mov ecx, 0xC0000080
        rdmsr
        or  eax, 1 << 8
        wrmsr

        ;; enable paging in the cr0 register
        mov eax, cr0
        or  eax, 1 << 31
        mov cr0, eax

        ;; So I know that this code is actually run.
        mov dword[0xb8004], 0x2f212f21
        
        ret

set_up_SSE:
        ;; Check for SSE and enable it. error with 'a' if it's unsupported
        ;; Rust/LLVM inserts SSE instructions sometimes. So we're enabling it.
        mov eax, 0x1
        cpuid
        test edx, 1<<25         ; Check for SSE
        jz .no_SSE

        ;; Enable SSE
        mov eax, cr0
        and ax,  0xfffB         ; clear coprocessor emulation CR0.EM
        or  ax,  0x2            ; set coprocessor monitoring CR0.MP
        mov cr0, eax            ;
        mov eax, cr4            ;
        or  ax,  3 << 9         ; set CR4.OSFXSR and CR4.OSMMEXCPT at the same time
        mov cr4, eax            ;

        ret
.no_SSE:
        mov al, "a"
        jmp error
        
section .rodata

struc GDTEntry
.limit1 resw 1
.basel  resw 1
.basem  resb 1
.access   resb 1
.flags_limith  resb 1
.baseh  resb 1        
endstruc

;; Lifted from Redox OS. MIT licenced so we can use it.
gdtr:
        dw gdt.end + 1          ; size
        dq gdt                  ; offset
        
gdt:
.null equ $ - gdt
        dq 0

.kernel_code: equ $ - gdt
        istruc GDTEntry
            at GDTEntry.limit1,       dw 0
            at GDTEntry.basel,        dw 0
            at GDTEntry.basem,        db 0
            at GDTEntry.access,       db attrib.present | attrib.user | attrib.code
            at GDTEntry.flags_limith, db flags.long_mode
            at GDTEntry.baseh,        db 0
        iend

.kernel_data: equ $ - gdt
        istruc GDTEntry
            at GDTEntry.limit1, dw 0
            at GDTEntry.basel,  dw 0
            at GDTEntry.basem,  db 0
            at GDTEntry.access, db attrib.present | attrib.user | attrib.writable
            at GDTEntry.flags_limith, db 0
            at GDTEntry.baseh,  db 0
        iend

;; .user_code: equ $ - gdt
;;         istruc GDTEntry
;;             at GDTEntry.limitl, dw 0
;;             at GDTEntry.basel,  dw 0
;;             at GDTEntry.basem,  db 0
;;             at GDTEntry.access, db attrib.present | attrib.ring3 | attrib.user | attrib.code
;;             at GDTEntry.flags_limith, db flags.long_mode
;;             at GDTEntry.baseh,  db 0
;;         iend

;; .user_data: equ $ - gdt
;;         istruc GDTEntry
;;             at GDTEntry.limitl, dw 0
;;             at GDTEntry.basel,  dw 0
;;             at GDTEntry.basem,  db 0
;;             at GDTEntry.access, db attrib.present | attrib.ring3 | attrib.user | attrib.writable
;;             at GDTEntry.flags_limith, db 0
;;             at GDTEntry.baseh, db 0
;;         iend


;; .tss: equ $ - gdt
;;         istruc GDTEntry
;;             at GDTEntry.limitl, dw (tss.end - tss) & 0xFFFF
;;             at GDTEntry.basel, dw (tss-$$+0x7C00) & 0xFFFF
;;             at GDTEntry.basem, db ((tss-$$+0x7C00) >> 16) & 0xFF
;;             at GDTEntry.access, db attrib.present | attrib.ring3 | attrib.tssAvailabe64
;;             at GDTEntry.flags_limith, db ((tss.end - tss) >> 16) & 0xF
;;             at GDTEntry.baseh, db ((tss-$$+0x7C00) >> 24) & 0xFF
;;         iend
;;         dq 0 ;tss descriptors are extended to 16 Bytes
        
.end: equ $ - gdt

    struc TSS
        .reserved1 resd 1    ;The previous TSS - if we used hardware task switching this would form a linked list.
        .rsp0 resq 1        ;The stack pointer to load when we change to kernel mode.
        .rsp1 resq 1        ;everything below here is unusued now..
        .rsp2 resq 1
        .reserved2 resd 1
        .reserved3 resd 1
        .ist1 resq 1
        .ist2 resq 1
        .ist3 resq 1
        .ist4 resq 1
        .ist5 resq 1
        .ist6 resq 1
        .ist7 resq 1
        .reserved4 resd 1
        .reserved5 resd 1
        .reserved6 resw 1
        .iomap_base resw 1
    endstruc

tss:
        istruc TSS
            at TSS.rsp0, dd 0x200000 - 128
            at TSS.iomap_base, dw 0xFFFF
        iend
.end:


        
;; Atrributes for gdt.
attrib:
    .present              equ 1 << 7
    .ring1                equ 1 << 5
    .ring2                equ 1 << 6
    .ring3                equ 1 << 5 | 1 << 6
    .user                 equ 1 << 4
;user
    .code                 equ 1 << 3
;   code
    .conforming           equ 1 << 2
    .readable             equ 1 << 1
;   data
    .expand_down          equ 1 << 2
    .writable             equ 1 << 1
    .accessed             equ 1 << 0
;system
;   legacy
    .tssAvailabe16        equ 0x1
    .ldt                  equ 0x2
    .tssBusy16            equ 0x3
    .call16               equ 0x4
    .task                 equ 0x5
    .interrupt16          equ 0x6
    .trap16               equ 0x7
    .tssAvailabe32        equ 0x9
    .tssBusy32            equ 0xB
    .call32               equ 0xC
    .interrupt32          equ 0xE
    .trap32               equ 0xF
;   long mode
    .ldt32                equ 0x2
    .tssAvailabe64        equ 0x9
    .tssBusy64            equ 0xB
    .call64               equ 0xC
    .interrupt64          equ 0xE
    .trap64               equ 0xF

flags:
    .granularity equ 1 << 7
    .available equ 1 << 4
;user
    .default_operand_size equ 1 << 6
;   code
    .long_mode equ 1 << 5
;   data
    .reserved equ 1 << 5
        
;; Back to Phill Ops code

;; The global data table
; gdt64:
;         ; Zero segment (required)
;         dq 0                            
; .code: equ $ - gdt64
;         ; kernel code segment
;         dd 0
;         dd (1<<12) | (1<<15) | (1<< 9) | (1<<11) | (1<<21)
;         ;dq (1<<44) | (1<<47) | (1<<41) | (1<<43) | (1<<53)
; .data: equ $ - gdt64
;         ; kernel data segment
;         dd 0
;         dd (1<<12) | (1<<15) | (1<< 9)
;         ; dq (1<<44) | (1<<47) | (1<<41)
; .pointer:
;         dw $ - gdt64 - 1
;         dq gdt64
        
section .bss
align 4096
p4_table:
        resb 4096
p3_table:
        resb 4096
p2_table:
        resb 4096
p1_table:
        resb 4096
stack_bottom:
        resb 8192
stack_top:
