use bums_macros::memsafe_multiversion;

// Provide a trivial Rust fallback
pub fn clobber_rbx_fallback() -> usize { 0 }

// This variant points at testdata/clobber_rbx.s which intentionally clobbers RBX
// The memsafe_multiversion proc-macro will attempt to prove the assembly safe and
// should fail for this variant, causing a compile-time error.
#[memsafe_multiversion(
    versions = [("clobber_rbx.s", "clobber_rbx", [])],
    fallback = clobber_rbx_fallback,
    invariants = [],
    abi_probe = false
)]
pub fn clobber_rbx() -> usize { unimplemented!() }

fn main() {
    // call to ensure symbol is referenced
    let _ = clobber_rbx();
}
