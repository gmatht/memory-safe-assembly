.intel_syntax noprefix
.text

/* Unrolled prefilter block (copied inline) */
.global internal_x86_code_v3_provable_prefilter_unrolled
.type internal_x86_code_v3_provable_prefilter_unrolled, @function
internal_x86_code_v3_provable_prefilter_unrolled:
    push rbp
    mov rbp, rsp
    mov rax, 32
    mov al, byte ptr [rdi + 0]
    cmp al, 0xE8
    je .CHK0
    cmp al, 0xE9
    je .CHK0
    jmp .NEXT0
.CHK0:
    mov dl, byte ptr [rdi + 4]
    cmp dl, 0
    je .NEXT0
    cmp dl, 0xFF
    je .NEXT0
    mov rax, 0
    pop rbp
    ret
.NEXT0:
    mov al, byte ptr [rdi + 1]
    cmp al, 0xE8
    je .CHK1
    cmp al, 0xE9
    je .CHK1
    jmp .NEXT1
.CHK1:
    mov dl, byte ptr [rdi + 5]
    cmp dl, 0
    je .NEXT1
    cmp dl, 0xFF
    je .NEXT1
    mov rax, 1
    pop rbp
    ret
.NEXT1:
    mov al, byte ptr [rdi + 2]
    cmp al, 0xE8
    je .CHK2
    cmp al, 0xE9
    je .CHK2
    jmp .NEXT2
.CHK2:
    mov dl, byte ptr [rdi + 6]
    cmp dl, 0
    je .NEXT2
    cmp dl, 0xFF
    je .NEXT2
    mov rax, 2
    pop rbp
    ret
.NEXT2:
    mov al, byte ptr [rdi + 3]
    cmp al, 0xE8
    je .CHK3
    cmp al, 0xE9
    je .CHK3
    jmp .NEXT3
.CHK3:
    mov dl, byte ptr [rdi + 7]
    cmp dl, 0
    je .NEXT3
    cmp dl, 0xFF
    je .NEXT3
    mov rax, 3
    pop rbp
    ret
.NEXT3:
    mov al, byte ptr [rdi + 4]
    cmp al, 0xE8
    je .CHK4
    cmp al, 0xE9
    je .CHK4
    jmp .NEXT4
.CHK4:
    mov dl, byte ptr [rdi + 8]
    cmp dl, 0
    je .NEXT4
    cmp dl, 0xFF
    je .NEXT4
    mov rax, 4
    pop rbp
    ret
.NEXT4:
    mov al, byte ptr [rdi + 5]
    cmp al, 0xE8
    je .CHK5
    cmp al, 0xE9
    je .CHK5
    jmp .NEXT5
.CHK5:
    mov dl, byte ptr [rdi + 9]
    cmp dl, 0
    je .NEXT5
    cmp dl, 0xFF
    je .NEXT5
    mov rax, 5
    pop rbp
    ret
.NEXT5:
    mov al, byte ptr [rdi + 6]
    cmp al, 0xE8
    je .CHK6
    cmp al, 0xE9
    je .CHK6
    jmp .NEXT6
.CHK6:
    mov dl, byte ptr [rdi + 10]
    cmp dl, 0
    je .NEXT6
    cmp dl, 0xFF
    je .NEXT6
    mov rax, 6
    pop rbp
    ret
.NEXT6:
    mov al, byte ptr [rdi + 7]
    cmp al, 0xE8
    je .CHK7
    cmp al, 0xE9
    je .CHK7
    jmp .NEXT7
.CHK7:
    mov dl, byte ptr [rdi + 11]
    cmp dl, 0
    je .NEXT7
    cmp dl, 0xFF
    je .NEXT7
    mov rax, 7
    pop rbp
    ret
.NEXT7:
    mov al, byte ptr [rdi + 8]
    cmp al, 0xE8
    je .CHK8
    cmp al, 0xE9
    je .CHK8
    jmp .NEXT8
.CHK8:
    mov dl, byte ptr [rdi + 12]
    cmp dl, 0
    je .NEXT8
    cmp dl, 0xFF
    je .NEXT8
    mov rax, 8
    pop rbp
    ret
.NEXT8:
    mov al, byte ptr [rdi + 9]
    cmp al, 0xE8
    je .CHK9
    cmp al, 0xE9
    je .CHK9
    jmp .NEXT9
.CHK9:
    mov dl, byte ptr [rdi + 13]
    cmp dl, 0
    je .NEXT9
    cmp dl, 0xFF
    je .NEXT9
    mov rax, 9
    pop rbp
    ret
.NEXT9:
    mov al, byte ptr [rdi + 10]
    cmp al, 0xE8
    je .CHK10
    cmp al, 0xE9
    je .CHK10
    jmp .NEXT10
.CHK10:
    mov dl, byte ptr [rdi + 14]
    cmp dl, 0
    je .NEXT10
    cmp dl, 0xFF
    je .NEXT10
    mov rax, 10
    pop rbp
    ret
.NEXT10:
    mov al, byte ptr [rdi + 11]
    cmp al, 0xE8
    je .CHK11
    cmp al, 0xE9
    je .CHK11
    jmp .NEXT11
.CHK11:
    mov dl, byte ptr [rdi + 15]
    cmp dl, 0
    je .NEXT11
    cmp dl, 0xFF
    je .NEXT11
    mov rax, 11
    pop rbp
    ret
.NEXT11:
    mov al, byte ptr [rdi + 12]
    cmp al, 0xE8
    je .CHK12
    cmp al, 0xE9
    je .CHK12
    jmp .NEXT12
.CHK12:
    mov dl, byte ptr [rdi + 16]
    cmp dl, 0
    je .NEXT12
    cmp dl, 0xFF
    je .NEXT12
    mov rax, 12
    pop rbp
    ret
.NEXT12:
    mov al, byte ptr [rdi + 13]
    cmp al, 0xE8
    je .CHK13
    cmp al, 0xE9
    je .CHK13
    jmp .NEXT13
.CHK13:
    mov dl, byte ptr [rdi + 17]
    cmp dl, 0
    je .NEXT13
    cmp dl, 0xFF
    je .NEXT13
    mov rax, 13
    pop rbp
    ret
.NEXT13:
    mov al, byte ptr [rdi + 14]
    cmp al, 0xE8
    je .CHK14
    cmp al, 0xE9
    je .CHK14
    jmp .NEXT14
.CHK14:
    mov dl, byte ptr [rdi + 18]
    cmp dl, 0
    je .NEXT14
    cmp dl, 0xFF
    je .NEXT14
    mov rax, 14
    pop rbp
    ret
.NEXT14:
    mov al, byte ptr [rdi + 15]
    cmp al, 0xE8
    je .CHK15
    cmp al, 0xE9
    je .CHK15
    jmp .NEXT15
.CHK15:
    mov dl, byte ptr [rdi + 19]
    cmp dl, 0
    je .NEXT15
    cmp dl, 0xFF
    je .NEXT15
    mov rax, 15
    pop rbp
    ret
.NEXT15:
    mov al, byte ptr [rdi + 16]
    cmp al, 0xE8
    je .CHK16
    cmp al, 0xE9
    je .CHK16
    jmp .NEXT16
.CHK16:
    mov dl, byte ptr [rdi + 20]
    cmp dl, 0
    je .NEXT16
    cmp dl, 0xFF
    je .NEXT16
    mov rax, 16
    pop rbp
    ret
.NEXT16:
    mov al, byte ptr [rdi + 17]
    cmp al, 0xE8
    je .CHK17
    cmp al, 0xE9
    je .CHK17
    jmp .NEXT17
.CHK17:
    mov dl, byte ptr [rdi + 21]
    cmp dl, 0
    je .NEXT17
    cmp dl, 0xFF
    je .NEXT17
    mov rax, 17
    pop rbp
    ret
.NEXT17:
    mov al, byte ptr [rdi + 18]
    cmp al, 0xE8
    je .CHK18
    cmp al, 0xE9
    je .CHK18
    jmp .NEXT18
.CHK18:
    mov dl, byte ptr [rdi + 22]
    cmp dl, 0
    je .NEXT18
    cmp dl, 0xFF
    je .NEXT18
    mov rax, 18
    pop rbp
    ret
.NEXT18:
    mov al, byte ptr [rdi + 19]
    cmp al, 0xE8
    je .CHK19
    cmp al, 0xE9
    je .CHK19
    jmp .NEXT19
.CHK19:
    mov dl, byte ptr [rdi + 23]
    cmp dl, 0
    je .NEXT19
    cmp dl, 0xFF
    je .NEXT19
    mov rax, 19
    pop rbp
    ret
.NEXT19:
    mov al, byte ptr [rdi + 20]
    cmp al, 0xE8
    je .CHK20
    cmp al, 0xE9
    je .CHK20
    jmp .NEXT20
.CHK20:
    mov dl, byte ptr [rdi + 24]
    cmp dl, 0
    je .NEXT20
    cmp dl, 0xFF
    je .NEXT20
    mov rax, 20
    pop rbp
    ret
.NEXT20:
    mov al, byte ptr [rdi + 21]
    cmp al, 0xE8
    je .CHK21
    cmp al, 0xE9
    je .CHK21
    jmp .NEXT21
.CHK21:
    mov dl, byte ptr [rdi + 25]
    cmp dl, 0
    je .NEXT21
    cmp dl, 0xFF
    je .NEXT21
    mov rax, 21
    pop rbp
    ret
.NEXT21:
    mov al, byte ptr [rdi + 22]
    cmp al, 0xE8
    je .CHK22
    cmp al, 0xE9
    je .CHK22
    jmp .NEXT22
.CHK22:
    mov dl, byte ptr [rdi + 26]
    cmp dl, 0
    je .NEXT22
    cmp dl, 0xFF
    je .NEXT22
    mov rax, 22
    pop rbp
    ret
.NEXT22:
    mov al, byte ptr [rdi + 23]
    cmp al, 0xE8
    je .CHK23
    cmp al, 0xE9
    je .CHK23
    jmp .NEXT23
.CHK23:
    mov dl, byte ptr [rdi + 27]
    cmp dl, 0
    je .NEXT23
    cmp dl, 0xFF
    je .NEXT23
    mov rax, 23
    pop rbp
    ret
.NEXT23:
    mov al, byte ptr [rdi + 24]
    cmp al, 0xE8
    je .CHK24
    cmp al, 0xE9
    je .CHK24
    jmp .NEXT24
.CHK24:
    mov dl, byte ptr [rdi + 28]
    cmp dl, 0
    je .NEXT24
    cmp dl, 0xFF
    je .NEXT24
    mov rax, 24
    pop rbp
    ret
.NEXT24:
    mov al, byte ptr [rdi + 25]
    cmp al, 0xE8
    je .CHK25
    cmp al, 0xE9
    je .CHK25
    jmp .NEXT25
.CHK25:
    mov dl, byte ptr [rdi + 29]
    cmp dl, 0
    je .NEXT25
    cmp dl, 0xFF
    je .NEXT25
    mov rax, 25
    pop rbp
    ret
.NEXT25:
    mov al, byte ptr [rdi + 26]
    cmp al, 0xE8
    je .CHK26
    cmp al, 0xE9
    je .CHK26
    jmp .NEXT26
.CHK26:
    mov dl, byte ptr [rdi + 30]
    cmp dl, 0
    je .NEXT26
    cmp dl, 0xFF
    je .NEXT26
    mov rax, 26
    pop rbp
    ret
.NEXT26:
    mov al, byte ptr [rdi + 27]
    cmp al, 0xE8
    je .CHK27
    cmp al, 0xE9
    je .CHK27
    jmp .NEXT27
.CHK27:
    mov dl, byte ptr [rdi + 31]
    cmp dl, 0
    je .NEXT27
    cmp dl, 0xFF
    je .NEXT27
    mov rax, 27
    pop rbp
    ret
.NEXT27:
    mov al, byte ptr [rdi + 28]
    cmp al, 0xE8
    je .CHK28
    cmp al, 0xE9
    je .CHK28
    jmp .NEXT28
.CHK28:
    jmp .NEXT28
.NEXT28:
    mov al, byte ptr [rdi + 29]
    cmp al, 0xE8
    je .CHK29
    cmp al, 0xE9
    je .CHK29
    jmp .NEXT29
.CHK29:
    jmp .NEXT29
.NEXT29:
    mov al, byte ptr [rdi + 30]
    cmp al, 0xE8
    je .CHK30
    cmp al, 0xE9
    je .CHK30
    jmp .NEXT30
.CHK30:
    jmp .NEXT30
.NEXT30:
    mov al, byte ptr [rdi + 31]
    cmp al, 0xE8
    je .CHK31
    cmp al, 0xE9
    je .CHK31
    jmp .NEXT31
.CHK31:
    jmp .NEXT31
.NEXT31:
    mov rax, 32
    pop rbp
    ret

/* Loop harness that repeatedly calls the unrolled block */
.global internal_x86_code_v3_provable_loop
.type internal_x86_code_v3_provable_loop, @function
internal_x86_code_v3_provable_loop:
    push rbp
    mov rbp, rsp
    /* rdi = base, rsi = len */
    mov rbx, rdi        /* save base in rbx (callee-saved) */
    xor rdx, rdx        /* offset = 0 */
.LOOP_START:
    cmp rsi, 32
    jb .LOOP_DONE
    lea rdi, [rbx + rdx]
    call internal_x86_code_v3_provable_prefilter_unrolled
    /* rax = local index (0..32) */
    cmp rax, 32
    jb .FOUND_LOCAL
    add rdx, 32
    sub rsi, 32
    jmp .LOOP_START
.FOUND_LOCAL:
    add rax, rdx
    pop rbp
    ret
.LOOP_DONE:
    /* return rdx (advanced offset) */
    mov rax, rdx
    pop rbp
    ret
