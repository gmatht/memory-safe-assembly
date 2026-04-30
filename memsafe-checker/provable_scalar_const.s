.intel_syntax noprefix
.text
.global internal_x86_code_v3_provable_chunk
.type internal_x86_code_v3_provable_chunk, @function
internal_x86_code_v3_provable_chunk:
    push rbp
    mov rbp, rsp
    /* rcx is pointer to a 32-byte chunk (p + idx) */
    mov al, byte ptr [rcx]    /* b = p[idx] */
    cmp al, 0xE8
    je .L_candidate
    cmp al, 0xE9
    je .L_candidate
    /* not a candidate: return 0 in rax */
    xor rax, rax
    pop rbp
    ret
.L_candidate:
    /* candidate: set rax=1 and return */
    mov rax, 1
    pop rbp
    ret
