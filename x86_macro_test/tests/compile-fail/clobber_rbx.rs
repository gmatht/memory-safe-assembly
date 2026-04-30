use bums_macros::memsafe_multiversion;

pub fn clobber_rbx_fallback() {}

#[memsafe_multiversion(
    versions = [("testdata/clobber_rbx.s", "clobber_rbx", [])],
    fallback = clobber_rbx_fallback,
    invariants = [],
    abi_probe = false
)]
pub fn clobber_rbx() { unimplemented!() }

fn main() {}
