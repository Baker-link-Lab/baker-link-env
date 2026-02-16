pub const APP_NAME: &str = "Baker link. Env";
pub const TUTORIAL_TEMPLATE: &str =
    "https://github.com/Baker-link-Lab/bakerlink_tutorial_template.git";

pub const GIT_HASH: &str = env!("GIT_HASH");
pub const GIT_TAG: &str = env!("GIT_TAG");

pub fn build_version_label() -> String {
    if GIT_TAG.is_empty() {
        format!("build {}", short_hash(GIT_HASH))
    } else {
        format!("{} ({})", GIT_TAG, short_hash(GIT_HASH))
    }
}

fn short_hash(hash: &str) -> &str {
    let len = hash.len().min(8);
    &hash[..len]
}
