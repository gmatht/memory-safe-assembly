use std::path::PathBuf;
use std::fs;

fn main() {
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    fs::create_dir_all(&out).unwrap();
    // NOTE: Original icx_core_avx2_v3_gas.s is workspace-specific; for tests we only
    // compile the small test assembly files under testdata/.

    // Also compile testdata simple impl for tests
    let src2 = PathBuf::from("testdata/simple_impl.s");
    let dst2 = out.join("simple_impl.s");
    let _ = fs::copy(&src2, &dst2).expect("copy asm2");
    cc::Build::new().file(dst2).flag_if_supported("-masm=intel").compile("simple_impl");

    // compile internal_x86_code_v3 for test wrapper linking
    let s3 = PathBuf::from("testdata/internal_x86_code_v3.s");
    let d3 = out.join("internal_x86_code_v3.s");
    let _ = fs::copy(&s3, &d3).expect("copy asm3");
    cc::Build::new().file(d3).flag_if_supported("-masm=intel").compile("internal_x86_code_v3");
    let s4 = PathBuf::from("testdata/internal_x86_code_v3_alt.s");
    let d4 = out.join("internal_x86_code_v3_alt.s");
    let _ = fs::copy(&s4, &d4).expect("copy asm4");
    cc::Build::new().file(d4).flag_if_supported("-masm=intel").compile("internal_x86_code_v3_alt");

    // compile clobber_rbx asm used by a proof-failure compile-fail test
    let s5 = PathBuf::from("testdata/clobber_rbx.s");
    let d5 = out.join("clobber_rbx.s");
    let _ = fs::copy(&s5, &d5).expect("copy asm5");
    cc::Build::new().file(d5).flag_if_supported("-masm=intel").compile("clobber_rbx");
}
