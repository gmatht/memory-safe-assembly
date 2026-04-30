use bums_macros::memsafe_multiversion;

// A trivial fallback
pub fn test_simple_impl_rust() -> usize { 0 }

#[memsafe_multiversion(
    versions = [("testdata/simple_impl.s", "test_simple_impl", [])],
    fallback = test_simple_impl_rust,
    invariants = [],
    abi_probe = false
)]
pub fn test_simple_impl() -> usize { unimplemented!() }

fn main() {
    let _ = test_simple_impl();
}
