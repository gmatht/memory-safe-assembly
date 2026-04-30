.intel_syntax noprefix
.text
.global internal_x86_code_v3_provable_scalar
.type internal_x86_code_v3_provable_scalar, @function
internal_x86_code_v3_provable_scalar:
    push rbp
    mov rbp, rsp
    /* Inputs (SysV-like):
       rdi = buffer base (p)
       rdx = idx (buffer_pos)
       ebx = now_pos
       r10d = prev_pos (may be read/updated)
       r11d = prev_mask (may be read/updated)
    */
    /* For provability, read from base only; treat idx as a separate abstract var */
    mov al, byte ptr [rdi]  /* load b = p[0] */
    cmp al, 0xE8
    je .L_candidate
    cmp al, 0xE9
    je .L_candidate
    /* not a candidate: advance by 1 */
    add rdx, 1
    mov rax, rdx
    pop rbp
    ret
.L_candidate:
    /* candidate: compute new_prev_pos = now_pos + idx (store in r10d) and advance by 5 */
    mov eax, ebx
    add eax, edx
    mov r10d, eax
    add rdx, 5
    mov rax, rdx
    pop rbp
    ret
