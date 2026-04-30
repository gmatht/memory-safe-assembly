.intel_syntax noprefix
.text
.extern internal_x86_code_v3_provable_prefilter_unrolled

.global internal_x86_code_v3_provable_loop_8
.type internal_x86_code_v3_provable_loop_8, @function
internal_x86_code_v3_provable_loop_8:
    push rbp
    mov rbp, rsp
    /* rdi = base, rsi = len */
    mov rbx, rdi
    xor rdx, rdx
    ; repeat 8 times calling the unrolled block
    .rept 8
        cmp rsi, 32
        jb .L_DONE_LOOP
        lea rdi, [rbx + rdx]
        call internal_x86_code_v3_provable_prefilter_unrolled
        cmp rax, 32
        jb .L_FOUND_LOOP
        add rdx, 32
        sub rsi, 32
    .endr
.L_DONE_LOOP:
    mov rax, rdx
    pop rbp
    ret
.L_FOUND_LOOP:
    add rax, rdx
    pop rbp
    ret
