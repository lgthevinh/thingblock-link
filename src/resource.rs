//! The served resource root — a single-version directory of installed packs
//! (block defs, generators, device manifests, vendored library sources) that the
//! helper exposes to the editor as static files.
//!
//! Flow 1 (resource serving): the static-file route serves this directory so the
//! browser can `import()` pack JS over HTTP. The browser is sandboxed and cannot
//! read the helper's filesystem, so an HTTP URL is the only handle it can use —
//! the machine path means nothing inside the page. This module owns just the
//! root's identity: validate and canonicalize it once at startup (fail fast if
//! absent) and hand its path to the route.
//!
//! Flow 2 (compile) will add `resolve_lib_dir`, turning a browser-supplied
//! `{pack, lib}` reference into a local library directory the arduino-cli daemon
//! reads in place. That consumer *is* a local process, so it uses the path
//! directly — the asymmetry that makes Flow 1 an HTTP serve and Flow 2 a
//! filesystem read of the same root.

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// The directory of installed packs the helper serves. Single version: its
/// contents are pinned to the helper install, so no per-pack version is tracked.
#[derive(Debug)]
pub struct ResourceRoot {
    /// Canonicalized at construction, so the static route gets a stable absolute
    /// path and later lib resolution can trust the root exists.
    root: PathBuf,
}

impl ResourceRoot {
    /// Validate and canonicalize the configured root, failing fast at startup with
    /// an actionable message if it is missing or is not a directory. Resolving the
    /// path here means the static route never serves through a dangling or
    /// relative root.
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let root = path.canonicalize().map_err(|e| {
            Error::Resource(format!(
                "resource root {} is unreadable: {e}",
                path.display()
            ))
        })?;
        if !root.is_dir() {
            return Err(Error::Resource(format!(
                "resource root {} is not a directory",
                root.display()
            )));
        }
        Ok(Self { root })
    }

    /// The canonical root directory, for the static-file route to serve from.
    pub fn path(&self) -> &Path {
        &self.root
    }
}
