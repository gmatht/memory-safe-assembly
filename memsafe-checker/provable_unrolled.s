.intel_syntax noprefix
.text
.global internal_x86_code_v3_provable_unrolled
.type internal_x86_code_v3_provable_unrolled, @function
internal_x86_code_v3_provable_unrolled:
    push rbp
    mov rbp, rsp
    /* rdi = buffer base
       This unrolled version checks offsets 0..31 for 0xE8/0xE9 and returns
       the index of first match in rax; if none, returns 32.
    */
    mov rax, 32
    mov al, byte ptr [rdi + 0]
    cmp al, 0xE8
    je .F0
    cmp al, 0xE9
    je .F0
    mov al, byte ptr [rdi + 1]
    cmp al, 0xE8
    je .F1
    cmp al, 0xE9
    je .F1
    mov al, byte ptr [rdi + 2]
    cmp al, 0xE8
    je .F2
    cmp al, 0xE9
    je .F2
    mov al, byte ptr [rdi + 3]
    cmp al, 0xE8
    je .F3
    cmp al, 0xE9
    je .F3
    mov al, byte ptr [rdi + 4]
    cmp al, 0xE8
    je .F4
    cmp al, 0xE9
    je .F4
    mov al, byte ptr [rdi + 5]
    cmp al, 0xE8
    je .F5
    cmp al, 0xE9
    je .F5
    mov al, byte ptr [rdi + 6]
    cmp al, 0xE8
    je .F6
    cmp al, 0xE9
    je .F6
    mov al, byte ptr [rdi + 7]
    cmp al, 0xE8
    je .F7
    cmp al, 0xE9
    je .F7
    mov al, byte ptr [rdi + 8]
    cmp al, 0xE8
    je .F8
    cmp al, 0xE9
    je .F8
    mov al, byte ptr [rdi + 9]
    cmp al, 0xE8
    je .F9
    cmp al, 0xE9
    je .F9
    mov al, byte ptr [rdi + 10]
    cmp al, 0xE8
    je .F10
    cmp al, 0xE9
    je .F10
    mov al, byte ptr [rdi + 11]
    cmp al, 0xE8
    je .F11
    cmp al, 0xE9
    je .F11
    mov al, byte ptr [rdi + 12]
    cmp al, 0xE8
    je .F12
    cmp al, 0xE9
    je .F12
    mov al, byte ptr [rdi + 13]
    cmp al, 0xE8
    je .F13
    cmp al, 0xE9
    je .F13
    mov al, byte ptr [rdi + 14]
    cmp al, 0xE8
    je .F14
    cmp al, 0xE9
    je .F14
    mov al, byte ptr [rdi + 15]
    cmp al, 0xE8
    je .F15
    cmp al, 0xE9
    je .F15
    mov al, byte ptr [rdi + 16]
    cmp al, 0xE8
    je .F16
    cmp al, 0xE9
    je .F16
    mov al, byte ptr [rdi + 17]
    cmp al, 0xE8
    je .F17
    cmp al, 0xE9
    je .F17
    mov al, byte ptr [rdi + 18]
    cmp al, 0xE8
    je .F18
    cmp al, 0xE9
    je .F18
    mov al, byte ptr [rdi + 19]
    cmp al, 0xE8
    je .F19
    cmp al, 0xE9
    je .F19
    mov al, byte ptr [rdi + 20]
    cmp al, 0xE8
    je .F20
    cmp al, 0xE9
    je .F20
    mov al, byte ptr [rdi + 21]
    cmp al, 0xE8
    je .F21
    cmp al, 0xE9
    je .F21
    mov al, byte ptr [rdi + 22]
    cmp al, 0xE8
    je .F22
    cmp al, 0xE9
    je .F22
    mov al, byte ptr [rdi + 23]
    cmp al, 0xE8
    je .F23
    cmp al, 0xE9
    je .F23
    mov al, byte ptr [rdi + 24]
    cmp al, 0xE8
    je .F24
    cmp al, 0xE9
    je .F24
    mov al, byte ptr [rdi + 25]
    cmp al, 0xE8
    je .F25
    cmp al, 0xE9
    je .F25
    mov al, byte ptr [rdi + 26]
    cmp al, 0xE8
    je .F26
    cmp al, 0xE9
    je .F26
    mov al, byte ptr [rdi + 27]
    cmp al, 0xE8
    je .F27
    cmp al, 0xE9
    je .F27
    mov al, byte ptr [rdi + 28]
    cmp al, 0xE8
    je .F28
    cmp al, 0xE9
    je .F28
    mov al, byte ptr [rdi + 29]
    cmp al, 0xE8
    je .F29
    cmp al, 0xE9
    je .F29
    mov al, byte ptr [rdi + 30]
    cmp al, 0xE8
    je .F30
    cmp al, 0xE9
    je .F30
    mov al, byte ptr [rdi + 31]
    cmp al, 0xE8
    je .F31
    cmp al, 0xE9
    je .F31
    /* none found */
    mov rax, 32
    pop rbp
    ret
.F0:
    mov rax, 0
    pop rbp
    ret
.F1:
    mov rax, 1
    pop rbp
    ret
.F2:
    mov rax, 2
    pop rbp
    ret
.F3:
    mov rax, 3
    pop rbp
    ret
.F4:
    mov rax, 4
    pop rbp
    ret
.F5:
    mov rax, 5
    pop rbp
    ret
.F6:
    mov rax, 6
    pop rbp
    ret
.F7:
    mov rax, 7
    pop rbp
    ret
.F8:
    mov rax, 8
    pop rbp
    ret
.F9:
    mov rax, 9
    pop rbp
    ret
.F10:
    mov rax, 10
    pop rbp
    ret
.F11:
    mov rax, 11
    pop rbp
    ret
.F12:
    mov rax, 12
    pop rbp
    ret
.F13:
    mov rax, 13
    pop rbp
    ret
.F14:
    mov rax, 14
    pop rbp
    ret
.F15:
    mov rax, 15
    pop rbp
    ret
.F16:
    mov rax, 16
    pop rbp
    ret
.F17:
    mov rax, 17
    pop rbp
    ret
.F18:
    mov rax, 18
    pop rbp
    ret
.F19:
    mov rax, 19
    pop rbp
    ret
.F20:
    mov rax, 20
    pop rbp
    ret
.F21:
    mov rax, 21
    pop rbp
    ret
.F22:
    mov rax, 22
    pop rbp
    ret
.F23:
    mov rax, 23
    pop rbp
    ret
.F24:
    mov rax, 24
    pop rbp
    ret
.F25:
    mov rax, 25
    pop rbp
    ret
.F26:
    mov rax, 26
    pop rbp
    ret
.F27:
    mov rax, 27
    pop rbp
    ret
.F28:
    mov rax, 28
    pop rbp
    ret
.F29:
    mov rax, 29
    pop rbp
    ret
.F30:
    mov rax, 30
    pop rbp
    ret
.F31:
    mov rax, 31
    pop rbp
    ret
