    .text
    .intel_syntax noprefix
    .global clobber_rbx
    .type clobber_rbx,@function
clobber_rbx:
    mov rbx, 0xdeadbeef
    ret
