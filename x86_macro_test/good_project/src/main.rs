use bums_macros::memsafe_multiversion;

// Trivial Rust fallback
pub fn safe_fn_rust() -> usize { 0 }

// This variant points at testdata/safe.s which is a tiny, well-behaved x86_64
// function (uses only eax/return value). The proc-macro should be able to
// prove it safe and expand the wrapper successfully.
#[memsafe_multiversion(
    versions = [("safe.s", "safe_fn", [])],
    fallback = safe_fn_rust,
    invariants = [],
    abi_probe = false
)]
pub fn safe_fn() -> usize { unimplemented!() }

fn main() {
    let _ = safe_fn();
}
