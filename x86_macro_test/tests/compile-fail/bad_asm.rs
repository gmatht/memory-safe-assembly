// This test triggers a parse-time error in the attribute (unknown key) and
// should therefore fail to compile quickly without invoking heavy proof work.
use bums_macros::memsafe_multiversion;

#[memsafe_multiversion(bogus = 1)]
pub fn should_fail() {}
