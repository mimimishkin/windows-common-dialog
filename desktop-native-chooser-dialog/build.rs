fn main() {
    if cfg!(feature = "java") {
        let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
        let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();

        let lib_path = match (target_os.as_str(), target_arch.as_str()) {
            ("linux", "x86_64") => "lib/linux/x64",
            ("linux", "aarch64") => "lib/linux/aarch64",
            ("windows", "x86_64") => "lib/windows/x64",
            ("windows", "aarch64") => "lib/windows/aarch64",
            ("macos", "x86_64") => "lib/macos/x64",
            ("macos", "aarch64") => "lib/macos/aarch64",
            _ => panic!("Cannot find appropriate jawt lib for linking in ./lib for platform: {}-{}", target_os, target_arch),
        };

        println!("cargo:rustc-link-search=native={lib_path}");
        println!("cargo:rustc-link-lib=jawt");
    }
}
