//! The environment the resolver runs in.
//!
//! The resolver algorithm itself performs no I/O and reads no configuration.
//! The handful of things it needs from the surrounding application come
//! through [`ResolverHost`], which makes the resolver's dependencies on its
//! environment explicit and lets it run without a [`GlobalContext`], e.g. in
//! tests. [`GlobalContextHost`](super::GlobalContextHost) is the implementation
//! Cargo uses.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::util::errors::CargoResult;
use crate::util::interning::InternedString;

/// What the resolver needs from the application embedding it.
///
/// Every method has a default, and `()` implements the trait, so callers that
/// need no integration can pass `&()`.
pub trait ResolverHost {
    /// Whether direct dependencies should resolve to the *minimum* version
    /// satisfying their requirement rather than the maximum.
    ///
    /// This corresponds to `-Zdirect-minimal-versions`.
    fn direct_minimal_versions(&self) -> bool {
        false
    }

    /// Time elapsed since this resolution started.
    ///
    /// Used to pace progress reporting and, in debug builds, to detect
    /// pathologically slow resolutions. Hosts without a monotonic clock can
    /// return [`Duration::ZERO`].
    fn elapsed(&self) -> Duration {
        Duration::ZERO
    }

    /// Called at most once, when resolution has been running for a noticeable
    /// amount of time, so the host can tell the user what is going on.
    fn report_slow_resolution(&self) -> CargoResult<()> {
        Ok(())
    }

    /// The flag that put the host into offline mode, if it is offline.
    ///
    /// Mentioned in error messages, e.g. `--offline`.
    fn offline_flag(&self) -> Option<&'static str> {
        None
    }

    /// Packages found at or below `path`, as `(name, manifest path)` pairs.
    ///
    /// Used to suggest corrections when a `path` dependency can't be found.
    /// Hosts without filesystem access can return an empty list.
    fn packages_at_path(&self, path: &Path) -> Vec<(InternedString, PathBuf)> {
        let _ = path;
        Vec::new()
    }
}

impl ResolverHost for () {}

impl<T: ResolverHost + ?Sized> ResolverHost for &T {
    fn direct_minimal_versions(&self) -> bool {
        (**self).direct_minimal_versions()
    }

    fn elapsed(&self) -> Duration {
        (**self).elapsed()
    }

    fn report_slow_resolution(&self) -> CargoResult<()> {
        (**self).report_slow_resolution()
    }

    fn offline_flag(&self) -> Option<&'static str> {
        (**self).offline_flag()
    }

    fn packages_at_path(&self, path: &Path) -> Vec<(InternedString, PathBuf)> {
        (**self).packages_at_path(path)
    }
}
