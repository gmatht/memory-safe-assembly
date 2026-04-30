.intel_syntax noprefix
.text
.global internal_x86_code_v3_provable_small
.type internal_x86_code_v3_provable_small, @function
internal_x86_code_v3_provable_small:
    push rbp
    mov rbp, rsp
    /* simple: read two 32-bit words from [rdi] and [rdi+4], add and return in rax */
    mov eax, dword ptr [rdi]
    add eax, dword ptr [rdi + 4]
    mov rax, rax
    pop rbp
    ret
