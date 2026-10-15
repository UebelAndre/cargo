//! `[patch]` entries and where they came from.

use std::fmt;
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};

use crate::context::Definition;
use crate::workspace::Dependency;

/// Patch is a dependency override that knows where it has been defined.
/// See [`PatchLocation`] for possible locations.
#[derive(Clone, Debug)]
pub struct Patch {
    pub dep: Dependency,
    pub loc: PatchLocation,
}

/// Place where a [`Patch`] has been defined.
#[derive(Clone, Debug)]
pub enum PatchLocation {
    /// Defined in a manifest.
    Manifest(PathBuf),
    /// Defined in cargo configuration.
    Config(Definition),
}

impl Display for PatchLocation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            PatchLocation::Manifest(p) => Path::display(p).fmt(f),
            PatchLocation::Config(def) => def.fmt(f),
        }
    }
}
