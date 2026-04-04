extern crate winres;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs");
    let git_hash = git_metadata();
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);

    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("icon/icon.ico");
        res.compile().unwrap();
    }
}

fn git_metadata() -> String {
    let root = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    run_git(&root, &["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".to_string())
}

fn run_git(root: &str, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}
