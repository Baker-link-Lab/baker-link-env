use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub const MAX_ARTIFACT_SIZE: u64 = 128 * 1024 * 1024;
pub const ARTIFACT_ROOT_ENV: &str = "BAKER_LINK_ARTIFACT_ROOT";
pub use probe_rs_tools::artifact_cache::{Usage, DEFAULT_LIMIT};

pub fn configure_limit(limit: u64) -> Result<Usage> {
    let root = root_dir()?;
    probe_rs_tools::artifact_cache::lock()?.set_limit(&root, limit)
}

pub fn clear_unused() -> Result<Usage> {
    let root = root_dir()?;
    probe_rs_tools::artifact_cache::lock()?.clear_unused(&root)
}

pub fn root_dir() -> Result<PathBuf> {
    let root = dirs::cache_dir()
        .context("Could not determine the operating system cache directory")?
        .join("baker-link-env")
        .join("artifacts");
    fs::create_dir_all(&root)
        .with_context(|| format!("Failed to create artifact cache at {}", root.display()))?;
    Ok(fs::canonicalize(root)?)
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
    let temporary = parent.join(format!(".{profile}.upload"));
    let upload = probe_rs_tools::artifact_cache::begin_upload(root, &temporary, content_length)?;
    let temporary = upload.path().to_path_buf();
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
        drop(file);
        let mut cache = probe_rs_tools::artifact_cache::lock()?;
        cache.prepare_store(root, &destination, received)?;
        #[cfg(target_os = "windows")]
        if destination.exists() {
            fs::remove_file(&destination)?;
        }
        fs::rename(&temporary, &destination)?;
        Ok(())
    })();

    if result.is_err() {
        if let Err(error) = fs::remove_file(&temporary) {
            if error.kind() != std::io::ErrorKind::NotFound {
                return Err(error).context("Failed to clean up incomplete artifact upload");
            }
        }
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

    fn put(root: &Path, project: &str, body: &[u8]) -> Result<PathBuf> {
        store(
            root,
            project,
            "debug",
            &format!("{:x}", Sha256::digest(body)),
            body.len() as u64,
            body,
        )
    }

    #[test]
    fn evicts_least_recently_used_and_counts_replacements() {
        let root = test_root();
        fs::create_dir_all(&root).unwrap();
        probe_rs_tools::artifact_cache::lock()
            .unwrap()
            .set_limit(&root, 10)
            .unwrap();
        let first = put(&root, "first", b"1234").unwrap();
        let second = put(&root, "second", b"5678").unwrap();
        fs::OpenOptions::new()
            .write(true)
            .open(&first)
            .unwrap()
            .set_times(fs::FileTimes::new().set_modified(UNIX_EPOCH))
            .unwrap();
        fs::OpenOptions::new()
            .write(true)
            .open(&second)
            .unwrap()
            .set_times(
                fs::FileTimes::new().set_modified(UNIX_EPOCH + std::time::Duration::from_secs(1)),
            )
            .unwrap();
        let lease = probe_rs_tools::artifact_cache::acquire(&first).unwrap();
        drop(lease);
        let third = put(&root, "third", b"abcd").unwrap();
        assert!(first.exists());
        assert!(!second.exists(), "last use must update eviction order");
        assert!(third.exists());
        put(&root, "third", b"abcdef").unwrap();
        let usage = probe_rs_tools::artifact_cache::lock()
            .unwrap()
            .usage(&root)
            .unwrap();
        assert_eq!((usage.bytes, usage.files, usage.limit), (10, 2, 10));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn active_artifacts_survive_eviction_and_clear_until_last_lease_is_dropped() {
        let root = test_root();
        fs::create_dir_all(&root).unwrap();
        probe_rs_tools::artifact_cache::lock()
            .unwrap()
            .set_limit(&root, 8)
            .unwrap();
        let first = put(&root, "first", b"1234").unwrap();
        let first_lease = probe_rs_tools::artifact_cache::acquire(&first).unwrap();
        let second_lease = probe_rs_tools::artifact_cache::acquire(&first).unwrap();
        let second = put(&root, "second", b"5678").unwrap();
        assert!(put(&root, "third", b"12345").is_err());
        assert!(
            second.exists(),
            "a rejected capacity request must not evict files"
        );
        let third = put(&root, "third", b"abcd").unwrap();
        assert!(!second.exists());
        let usage = probe_rs_tools::artifact_cache::lock()
            .unwrap()
            .clear_unused(&root)
            .unwrap();
        assert_eq!((usage.files, usage.active_files, usage.bytes), (1, 1, 4));
        assert!(!third.exists());
        drop(first_lease);
        assert_eq!(
            probe_rs_tools::artifact_cache::lock()
                .unwrap()
                .clear_unused(&root)
                .unwrap()
                .files,
            1
        );
        drop(second_lease);
        assert_eq!(
            probe_rs_tools::artifact_cache::lock()
                .unwrap()
                .clear_unused(&root)
                .unwrap()
                .files,
            0
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn restart_upload_replaces_pinned_artifact_without_evicting_it() {
        let root = test_root();
        fs::create_dir_all(&root).unwrap();
        probe_rs_tools::artifact_cache::lock()
            .unwrap()
            .set_limit(&root, 8)
            .unwrap();
        let path = put(&root, "first", b"old").unwrap();
        let lease = probe_rs_tools::artifact_cache::acquire(&path).unwrap();
        put(&root, "first", b"new-elf").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"new-elf");
        assert_eq!(
            probe_rs_tools::artifact_cache::lock()
                .unwrap()
                .clear_unused(&root)
                .unwrap()
                .active_files,
            1
        );
        drop(lease);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bad_upload_does_not_evict_and_limit_reduction_protects_active_files() {
        let root = test_root();
        fs::create_dir_all(&root).unwrap();
        probe_rs_tools::artifact_cache::lock()
            .unwrap()
            .set_limit(&root, 8)
            .unwrap();
        let path = put(&root, "first", b"12345678").unwrap();
        let lease = probe_rs_tools::artifact_cache::acquire(&path).unwrap();
        assert!(store(
            &root,
            "second",
            "debug",
            &"0".repeat(64),
            8,
            &b"87654321"[..]
        )
        .is_err());
        assert!(path.exists());
        assert!(!root.join("second/.debug.upload").exists());
        assert!(probe_rs_tools::artifact_cache::lock()
            .unwrap()
            .set_limit(&root, 4)
            .is_err());
        assert_eq!(
            probe_rs_tools::artifact_cache::lock().unwrap().limit(&root),
            8
        );
        drop(lease);
        let usage = probe_rs_tools::artifact_cache::lock()
            .unwrap()
            .set_limit(&root, 4)
            .unwrap();
        assert_eq!(usage.bytes, 0);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn startup_removes_stale_uploads_but_has_no_age_expiry() {
        let root = test_root();
        let path = put(&root, "first", b"1234").unwrap();
        fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .unwrap()
            .set_times(fs::FileTimes::new().set_modified(UNIX_EPOCH))
            .unwrap();
        fs::write(root.join("first/.debug.upload"), b"interrupted").unwrap();
        fs::write(root.join("first/notes.txt"), b"keep").unwrap();
        let usage = probe_rs_tools::artifact_cache::lock()
            .unwrap()
            .set_limit(&root, DEFAULT_LIMIT)
            .unwrap();
        assert_eq!(usage.limit, 1024 * 1024 * 1024);
        assert_eq!((usage.bytes, usage.files), (4, 1));
        assert!(path.exists());
        assert!(!root.join("first/.debug.upload").exists());
        probe_rs_tools::artifact_cache::lock()
            .unwrap()
            .clear_unused(&root)
            .unwrap();
        assert!(root.join("first/notes.txt").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn default_one_gib_limit_is_enforced_at_the_exact_boundary() {
        let root = test_root();
        let path = artifact_path(&root, "existing", "debug").unwrap();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::File::create(&path)
            .unwrap()
            .set_len(DEFAULT_LIMIT)
            .unwrap();
        let usage = probe_rs_tools::artifact_cache::lock()
            .unwrap()
            .set_limit(&root, DEFAULT_LIMIT)
            .unwrap();
        assert_eq!(usage.bytes, 1_073_741_824);
        assert!(path.exists(), "the exact limit must be allowed");
        let new_path = put(&root, "new", b"x").unwrap();
        assert!(
            !path.exists(),
            "one byte over capacity must trigger eviction"
        );
        let usage = probe_rs_tools::artifact_cache::lock()
            .unwrap()
            .usage(&root)
            .unwrap();
        assert_eq!((usage.bytes, usage.files), (1, 1));
        assert!(new_path.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn maintenance_does_not_remove_in_progress_uploads() {
        let root = test_root();
        let temporary = root.join("example/.debug.upload");
        let upload = probe_rs_tools::artifact_cache::begin_upload(&root, &temporary, 4).unwrap();
        probe_rs_tools::artifact_cache::lock()
            .unwrap()
            .clear_unused(&root)
            .unwrap();
        assert!(
            temporary.parent().unwrap().is_dir(),
            "pending upload directory must survive maintenance"
        );
        fs::write(root.join("example/old.elf"), b"old").unwrap();
        probe_rs_tools::artifact_cache::lock()
            .unwrap()
            .clear_unused(&root)
            .unwrap();
        assert!(
            temporary.parent().unwrap().is_dir(),
            "eviction must preserve pending upload directories"
        );
        fs::write(&temporary, b"1234").unwrap();
        assert!(probe_rs_tools::artifact_cache::begin_upload(&root, &temporary, 4).is_err());
        probe_rs_tools::artifact_cache::lock()
            .unwrap()
            .set_limit(&root, 8)
            .unwrap();
        probe_rs_tools::artifact_cache::lock()
            .unwrap()
            .clear_unused(&root)
            .unwrap();
        assert!(temporary.exists());
        drop(upload);
        probe_rs_tools::artifact_cache::lock()
            .unwrap()
            .clear_unused(&root)
            .unwrap();
        assert!(!temporary.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_path_casing_does_not_bypass_active_protection_or_replacement_accounting() {
        let root = test_root();
        fs::create_dir_all(&root).unwrap();
        probe_rs_tools::artifact_cache::lock()
            .unwrap()
            .set_limit(&root, 8)
            .unwrap();
        let path = put(&root, "Example", b"original").unwrap();
        let lease =
            probe_rs_tools::artifact_cache::acquire(&root.join("example/debug.elf")).unwrap();
        put(&root, "EXAMPLE", b"replaced").unwrap();
        store(
            &root,
            "EXAMPLE",
            "DEBUG",
            &format!("{:x}", Sha256::digest(b"replaced")),
            8,
            &b"replaced"[..],
        )
        .unwrap();
        let usage = probe_rs_tools::artifact_cache::lock()
            .unwrap()
            .clear_unused(&root)
            .unwrap();
        assert_eq!((usage.bytes, usage.active_files), (8, 1));
        assert_eq!(fs::read(&path).unwrap(), b"replaced");
        drop(lease);
        fs::remove_dir_all(root).unwrap();
    }
}
