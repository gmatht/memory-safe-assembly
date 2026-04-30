.intel_syntax noprefix
.text
.extern internal_x86_code_v3_provable_prefilter_unrolled

.global internal_x86_code_v3_provable_loop_4
.type internal_x86_code_v3_provable_loop_4, @function
internal_x86_code_v3_provable_loop_4:
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
    sub rsi, 32
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
