.intel_syntax noprefix
.text

/* reuse the same unrolled block symbol name defined elsewhere */
.extern internal_x86_code_v3_provable_prefilter_unrolled

.global internal_x86_code_v3_provable_loop_small
.type internal_x86_code_v3_provable_loop_small, @function
internal_x86_code_v3_provable_loop_small:
    push rbp
    mov rbp, rsp
    /* rdi = base, rsi = len */
    mov rbx, rdi
    xor rdx, rdx
    cmp rsi, 32
    jb .DONE
    lea rdi, [rbx + rdx]
    call internal_x86_code_v3_provable_prefilter_unrolled
    cmp rax, 32
    jb .FOUND
    add rdx, 32
    sub rsi, 32
    cmp rsi, 32
    jb .DONE
    lea rdi, [rbx + rdx]
    call internal_x86_code_v3_provable_prefilter_unrolled
    cmp rax, 32
    jb .FOUND
    add rdx, 32
.DONE:
    mov rax, rdx
    pop rbp
    ret
.FOUND:
    add rax, rdx
    pop rbp
    ret
