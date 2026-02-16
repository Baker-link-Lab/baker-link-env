extern crate winres;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs");
    let (git_hash, git_tag) = git_metadata();
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
    println!("cargo:rustc-env=GIT_TAG={}", git_tag);

    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("icon/icon.ico");
        res.compile().unwrap();
    }
}

fn git_metadata() -> (String, String) {
    let root = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let hash = run_git(&root, &["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".to_string());
    let tag = run_git(&root, &["describe", "--tags", "--match", "v*.*.*", "--abbrev=0"])
        .unwrap_or_default();
    (hash, tag)
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
