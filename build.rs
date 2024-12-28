use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=LLVM_CONFIG");

    let llvm_config = find_llvm_config().expect(
        "Could not find llvm-config. Please install LLVM or set LLVM_CONFIG environment variable",
    );

    // Get LLVM version
    let output = Command::new(&llvm_config)
        .arg("--version")
        .output()
        .expect("Failed to execute llvm-config");

    let version = String::from_utf8_lossy(&output.stdout);
    println!("cargo:warning=Using LLVM version {}", version.trim());

    // Set LLVM flags
    println!(
        "cargo:rustc-link-search=native={}",
        get_llvm_lib_dir(&llvm_config)
    );

    for lib in get_llvm_libs(&llvm_config) {
        println!("cargo:rustc-link-lib=static={}", lib);
    }

    // Generate build info
    generate_build_info();
}

fn find_llvm_config() -> Option<String> {
    // First check environment variable
    if let Ok(config) = env::var("LLVM_CONFIG") {
        return Some(config);
    }

    // Then try standard locations
    let candidates = if cfg!(target_os = "windows") {
        vec!["llvm-config.exe"]
    } else {
        vec![
            "llvm-config-15",
            "llvm-config-14",
            "llvm-config-13",
            "llvm-config",
        ]
    };

    for candidate in candidates {
        if let Ok(output) = Command::new(candidate).arg("--version").output() {
            if output.status.success() {
                return Some(candidate.to_string());
            }
        }
    }

    None
}

fn get_llvm_lib_dir(llvm_config: &str) -> String {
    let output = Command::new(llvm_config)
        .arg("--libdir")
        .output()
        .expect("Failed to get LLVM lib directory");

    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn get_llvm_libs(llvm_config: &str) -> Vec<String> {
    let output = Command::new(llvm_config)
        .args(["--libs", "core", "codegen", "mc", "support"])
        .output()
        .expect("Failed to get LLVM libraries");

    String::from_utf8_lossy(&output.stdout)
        .trim()
        .split(' ')
        .map(|lib| lib.strip_prefix("-l").unwrap_or(lib).trim().to_string())
        .collect()
}

fn generate_build_info() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let build_info = format!(
        r#"pub const BUILD_INFO: &str = "Built on {} with rustc {}";"#,
        env!("CARGO_PKG_VERSION"),
        rustc_version()
    );

    std::fs::write(out_dir.join("build_info.rs"), build_info).expect("Failed to write build info");
}

fn rustc_version() -> String {
    let output = Command::new("rustc")
        .arg("--version")
        .output()
        .expect("Failed to get rustc version");

    String::from_utf8_lossy(&output.stdout).trim().to_string()
}
