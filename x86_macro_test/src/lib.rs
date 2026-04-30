use bums_macros::memsafe_multiversion;

// Provide a fallback Rust implementation
pub fn internal_x86_code_v3_rust(buf: *mut u8, buf_len: usize, _is_encoder: i32) -> usize {
    // trivial safe fallback that does nothing
    if buf_len == 0 || buf.is_null() {
        return 0;
    }
    unsafe { (*buf) as usize }
}

// Fallback for clobber_rbx runtime ABI-probe test
// (removed runtime ABI-probe wrapper to avoid macro expansion type inference issues in tests)
pub fn clobber_rbx_fallback(_buf: *mut u8, _buf_len: usize) -> usize {
    // no-op fallback
    0usize
}


#[memsafe_multiversion(
    versions = [("internal_x86_code_v3.s", "internal_x86_code_v3", [])],
    fallback = internal_x86_code_v3_rust,
    invariants = [ buf_len >= 5 ],
    abi_probe = false
)]
pub fn internal_x86_code_v3(buf: *mut u8, buf_len: usize, is_encoder: i32) -> usize {
    unimplemented!()
}

// Multi-variant wrapper: prefer alt (returns 7) first, then main (42), then fallback.
#[memsafe_multiversion(
    versions = [
        ("internal_x86_code_v3_alt.s", "internal_x86_code_v3_alt", []),
        ("internal_x86_code_v3.s", "internal_x86_code_v3", []),
    ],
    fallback = internal_x86_code_v3_rust,
    invariants = [ buf_len >= 1 ],
    abi_probe = false
)]
pub fn internal_x86_code_v3_multi(buf: *mut u8, buf_len: usize, is_encoder: i32) -> usize {
    unimplemented!()
}

// Runtime ABI-probe wrapper for clobbering candidate: the asm intentionally clobbers RBX.
// Probe should detect clobber and cause the wrapper to use the Rust fallback.
#[memsafe_multiversion(
    versions = [("clobber_rbx.s", "clobber_rbx", [])],
    fallback = clobber_rbx_fallback,
    invariants = [],
    abi_probe = true,
    abi_probe_sample_size = 16
)]
pub fn clobber_rbx(buf: *mut u8, buf_len: usize) -> usize {
    unimplemented!()
}
