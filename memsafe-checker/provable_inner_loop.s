.intel_syntax noprefix
.text
.global internal_x86_code_v3_provable_inner
.type internal_x86_code_v3_provable_inner, @function
internal_x86_code_v3_provable_inner:
    push rbp
    mov rbp, rsp
    /* Inputs:
       rdi = buffer base (p)
       rdx = base idx
       r8  = limit (optional)
    */
    mov r9, rdx        /* r9 = scanning index */
    mov r8, 32         /* scan up to 32 bytes */
.L_scan_loop:
    cmp r8, 0
    je .L_not_found
    mov bl, byte ptr [rdi + r9]
    cmp bl, 0xE8
    je .L_found
    cmp bl, 0xE9
    je .L_found
    inc r9
    sub r8, 1
    jmp .L_scan_loop
.L_not_found:
    /* return rdx + 32 (advance by block size) */
    lea rax, [rdx + 32]
    pop rbp
    ret
.L_found:
    /* return r9 (the index of found candidate) */
    mov rax, r9
    pop rbp
    ret
