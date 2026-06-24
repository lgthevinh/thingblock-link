//! A minimal self-cleaning temporary directory.
//!
//! Creates a uniquely-named directory under the OS temp dir and removes it
//! (recursively) on drop — enough for our scratch space (materialized sketches
//! and their compiled artifacts) without pulling in a temp-file crate.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use tracing::debug;

/// Disambiguates temp dirs created in quick succession within one process.
static SEQ: AtomicU64 = AtomicU64::new(0);

/// A temporary directory removed (recursively) when this handle is dropped.
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    /// Create `<os-temp>/<prefix>-<pid>-<nanos>-<seq>`. The pid/nanos/seq triple
    /// makes the name unique across processes and concurrent callers.
    pub fn new(prefix: &str) -> std::io::Result<Self> {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let seq = SEQ.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("{prefix}-{}-{nanos}-{seq}", std::process::id()));
        std::fs::create_dir_all(&path)?;
        Ok(Self { path })
    }

    /// The directory's path.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        // Best-effort: the OS temp sweep is the backstop if removal fails.
        if let Err(e) = std::fs::remove_dir_all(&self.path) {
            debug!(path = %self.path.display(), error = %e, "failed to remove temp dir");
        }
    }
}
