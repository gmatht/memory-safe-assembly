    .text
    .intel_syntax noprefix
    .global test_simple_impl
    .type test_simple_impl,@function
test_simple_impl:
    mov eax, 42
    ret
