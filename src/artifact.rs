use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub const MAX_ARTIFACT_SIZE: u64 = 128 * 1024 * 1024;
pub const ARTIFACT_ROOT_ENV: &str = "BAKER_LINK_ARTIFACT_ROOT";

pub fn root_dir() -> Result<PathBuf> {
    let root = dirs::cache_dir()
        .context("Could not determine the operating system cache directory")?
        .join("baker-link-env")
        .join("artifacts");
    fs::create_dir_all(&root)
        .with_context(|| format!("Failed to create artifact cache at {}", root.display()))?;
    Ok(root)
}

pub fn artifact_path(root: &Path, project: &str, profile: &str) -> Result<PathBuf> {
    validate_id("project", project)?;
    validate_id("profile", profile)?;
    Ok(root.join(project).join(format!("{profile}.elf")))
}

pub fn store<R: Read>(
    root: &Path,
    project: &str,
    profile: &str,
    expected_sha256: &str,
    content_length: u64,
    mut reader: R,
) -> Result<PathBuf> {
    if content_length > MAX_ARTIFACT_SIZE {
        bail!(
            "Artifact exceeds the {} MiB limit",
            MAX_ARTIFACT_SIZE / 1024 / 1024
        );
    }
    validate_sha256(expected_sha256)?;

    let destination = artifact_path(root, project, profile)?;
    let parent = destination
        .parent()
        .context("Artifact path has no parent")?;
    fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(".{profile}.upload"));
    let result = (|| -> Result<()> {
        let mut file = fs::File::create(&temporary)?;
        let mut hasher = Sha256::new();
        let mut limited = reader.by_ref().take(content_length + 1);
        let mut buffer = [0_u8; 64 * 1024];
        let mut received = 0_u64;

        loop {
            let count = limited.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            received += count as u64;
            if received > content_length || received > MAX_ARTIFACT_SIZE {
                bail!("Artifact body is larger than Content-Length");
            }
            hasher.update(&buffer[..count]);
            file.write_all(&buffer[..count])?;
        }
        if received != content_length {
            bail!(
                "Artifact body is incomplete: expected {content_length} bytes, received {received}"
            );
        }
        file.sync_all()?;

        let actual = format!("{:x}", hasher.finalize());
        if !actual.eq_ignore_ascii_case(expected_sha256) {
            bail!("Artifact SHA-256 mismatch");
        }
        #[cfg(target_os = "windows")]
        if destination.exists() {
            fs::remove_file(&destination)?;
        }
        fs::rename(&temporary, &destination)?;
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result?;
    Ok(destination)
}

fn validate_id(label: &str, value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 80
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        bail!("Invalid {label} identifier");
    }
    Ok(())
}

fn validate_sha256(value: &str) -> Result<()> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("Invalid SHA-256 header");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_root() -> PathBuf {
        std::env::temp_dir().join(format!(
            "baker-link-artifact-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn stores_verified_artifact() {
        let root = test_root();
        let body = b"test-elf";
        let checksum = format!("{:x}", Sha256::digest(body));
        let path = store(
            &root,
            "example",
            "debug",
            &checksum,
            body.len() as u64,
            &body[..],
        )
        .unwrap();

        assert_eq!(fs::read(path).unwrap(), body);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_traversal_and_bad_checksum() {
        let root = test_root();
        assert!(artifact_path(&root, "../escape", "debug").is_err());
        assert!(store(&root, "example", "debug", "bad", 0, &[][..]).is_err());
        assert!(!root.join("example/debug.elf").exists());
    }
}
