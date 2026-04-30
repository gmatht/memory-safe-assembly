use std::path::PathBuf;
use std::fs;

fn main() {
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    fs::create_dir_all(&out).unwrap();

    // copy and assemble the safe asm into OUT_DIR so the proc-macro can open it
    let src = PathBuf::from("testdata/safe.s");
    let dst = out.join("safe.s");
    let _ = fs::copy(&src, &dst).expect("copy asm");
    cc::Build::new().file(dst).flag_if_supported("-masm=intel").compile("safe");
}
