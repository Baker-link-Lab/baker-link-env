pub const APP_NAME: &str = "Baker link. Env";
pub const TUTORIAL_TEMPLATE: &str =
    "https://github.com/Baker-link-Lab/bakerlink_tutorial_template.git";
pub const TEMPLATE_URL: &str = "https://github.com/Baker-link-Lab/bakerlink_tutorial_template";

pub const GIT_HASH: &str = env!("GIT_HASH");
pub const GIT_TAG: &str = env!("GIT_TAG");

pub fn build_version_label() -> String {
    let short = &GIT_HASH[..GIT_HASH.len().min(8)];
    if GIT_TAG.is_empty() {
        format!("build {}", short)
    } else {
        format!("{} ({})", GIT_TAG, short)
    }
}
