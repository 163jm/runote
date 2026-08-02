use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=assets/app.rc");
    println!("cargo:rerun-if-changed=assets/app.ico");
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let out_file = out.join("app_res.o");
    let status = std::process::Command::new("windres")
        .arg("assets/app.rc")
        .arg("-O")
        .arg("coff")
        .arg("-o")
        .arg(&out_file)
        .status()
        .expect("windres 未找到：请确保 MinGW 的 bin 目录在 PATH 中");
    assert!(status.success(), "windres 编译图标资源失败");
    println!("cargo:rustc-link-arg={}", out_file.display());
}
