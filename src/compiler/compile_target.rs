//! The target a compilation is for.

use std::path::Path;
use std::path::PathBuf;

use anyhow::Context as _;
use anyhow::bail;
use serde::Serialize;

use crate::util::errors::CargoResult;
use crate::util::interning::InternedString;

/// Abstraction for the representation of a compilation target that Cargo has.
///
/// Compilation targets are one of two things right now:
///
/// 1. A raw target string, like `x86_64-unknown-linux-gnu`.
/// 2. The path to a JSON file, such as `/path/to/my-target.json`.
///
/// Raw target strings are typically dictated by `rustc` itself and represent
/// built-in targets. Custom JSON files are somewhat unstable, but supported
/// here in Cargo. Note that for JSON target files this `CompileTarget` stores a
/// full canonicalized path to the target.
///
/// The main reason for this existence is to handle JSON target files where when
/// we call rustc we pass full paths but when we use it for Cargo's purposes
/// like naming directories or looking up configuration keys we only check the
/// file stem of JSON target files. For built-in rustc targets this is just an
/// uninterpreted string basically.
#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy, PartialOrd, Ord, Serialize)]
pub enum CompileTarget {
    Tuple(InternedString),
    Json {
        short: InternedString,
        path: InternedString,
    },
}

impl CompileTarget {
    pub fn new(name: &str, unstable_json: bool) -> CargoResult<CompileTarget> {
        let name = name.trim();
        if name.is_empty() {
            bail!("target was empty");
        }
        if !name.ends_with(".json") {
            return Ok(CompileTarget::Tuple(name.into()));
        }

        if !unstable_json {
            bail!(
                "`.json` target specs require -Zjson-target-spec to be added to the cargo invocation"
            );
        }

        // If `name` ends in `.json` then it's likely a custom target
        // specification. Canonicalize the path to ensure that different builds
        // with different paths always produce the same result.
        let p = try_canonicalize(Path::new(name))
            .with_context(|| format!("target path `{name}` is not a valid file"))?;
        let path = p
            .to_str()
            .ok_or_else(|| anyhow::format_err!("target path `{name}` is not valid unicode"))?
            .into();
        let short = p.file_stem().unwrap().to_str().unwrap().into();
        Ok(CompileTarget::Json { short, path })
    }

    /// Returns the full unqualified name of this target, suitable for passing
    /// to `rustc` directly.
    ///
    /// Typically this is pretty much the same as `short_name`, but for the case
    /// of JSON target files this will be a full canonicalized path name for the
    /// current filesystem.
    pub fn rustc_target(&self) -> InternedString {
        match self {
            CompileTarget::Tuple(name) => *name,
            CompileTarget::Json { path, .. } => *path,
        }
    }

    /// Returns a "short" version of the target name suitable for usage within
    /// Cargo for configuration and such.
    ///
    /// This is typically the same as `rustc_target`, or the full name, but for
    /// JSON target files this returns just the file stem (e.g. `foo` out of
    /// `foo.json`) instead of the full path.
    pub fn short_name(&self) -> &str {
        match self {
            CompileTarget::Tuple(name) => name,
            CompileTarget::Json { short, .. } => short,
        }
    }
}

#[cfg(not(windows))]
#[inline]
fn try_canonicalize<P: AsRef<Path>>(path: P) -> std::io::Result<PathBuf> {
    std::fs::canonicalize(&path)
}

/// Like [`std::fs::canonicalize`], but handles paths that exist but cannot be
/// canonicalized on Windows (e.g. on some network drives) by making them
/// absolute instead.
#[cfg(windows)]
#[inline]
fn try_canonicalize<P: AsRef<Path>>(path: P) -> std::io::Result<PathBuf> {
    use std::io::Error;
    use std::io::ErrorKind;

    std::fs::canonicalize(&path).or_else(|_| {
        if !path.as_ref().try_exists()? {
            return Err(Error::new(ErrorKind::NotFound, "the path was not found"));
        }
        std::path::absolute(&path)
    })
}
