use anyhow::bail;
use std::path::Path;

pub fn ensure_existing_file(path: &Path, label: &str) -> anyhow::Result<()> {
    if !path.exists() {
        bail!("{} does not exist: {}", label, path.display());
    }
    if !path.is_file() {
        bail!("{} is not a regular file: {}", label, path.display());
    }
    Ok(())
}
