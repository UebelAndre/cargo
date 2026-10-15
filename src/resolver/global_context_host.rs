//! The [`ResolverHost`] Cargo itself uses.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::context::GlobalContext;
use crate::resolver::ResolverHost;
use crate::sources::path::RecursivePathSource;
use crate::util::errors::CargoResult;
use crate::util::interning::InternedString;
use crate::workspace::SourceId;

/// The [`ResolverHost`] backed by a [`GlobalContext`]: unstable flags,
/// the shell, offline mode and the filesystem.
pub struct GlobalContextHost<'gctx> {
    gctx: &'gctx GlobalContext,
    start: Instant,
}

impl<'gctx> GlobalContextHost<'gctx> {
    /// Creates a host; `elapsed` is measured from this point.
    pub fn new(gctx: &'gctx GlobalContext) -> Self {
        Self {
            gctx,
            start: Instant::now(),
        }
    }
}

impl ResolverHost for GlobalContextHost<'_> {
    fn direct_minimal_versions(&self) -> bool {
        self.gctx.cli_unstable().direct_minimal_versions
    }

    fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    fn report_slow_resolution(&self) -> CargoResult<()> {
        // Only when stderr is a tty (a human is likely to be watching) to
        // ensure deterministic output when observed by tools.
        if self.gctx.shell().is_err_tty() {
            self.gctx
                .shell()
                .status("Resolving", "dependency graph...")?;
        }
        Ok(())
    }

    fn offline_flag(&self) -> Option<&'static str> {
        self.gctx.offline_flag()
    }

    fn packages_at_path(&self, path: &Path) -> Vec<(InternedString, PathBuf)> {
        if !path.is_dir() {
            return Vec::new();
        }
        let Ok(source_id) = SourceId::for_path(path) else {
            return Vec::new();
        };
        let source = RecursivePathSource::new(path, source_id, self.gctx);
        source
            .read_packages()
            .map(|packages| {
                packages
                    .into_iter()
                    .map(|pkg| (pkg.name(), pkg.manifest_path().to_path_buf()))
                    .collect()
            })
            .unwrap_or_default()
    }
}
