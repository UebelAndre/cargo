//! Type definitions for cross-compilation.

use crate::util::errors::CargoResult;
use crate::util::{GlobalContext, StableHasher};
use crate::workspace::Target;
use crate::workspace::dependency::ArtifactTarget;

pub use super::compile_target::CompileTarget;
use cargo_util::ProcessBuilder;
use std::collections::BTreeSet;
use std::fs;
use std::hash::{Hash, Hasher};

/// Indicator for how a unit is being compiled.
///
/// This is used primarily for organizing cross compilations vs host
/// compilations, where cross compilations happen at the request of `--target`
/// and host compilations happen for things like build scripts and procedural
/// macros.
#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy, PartialOrd, Ord)]
pub enum CompileKind {
    /// Attached to a unit that is compiled for the "host" system or otherwise
    /// is compiled without a `--target` flag. This is used for procedural
    /// macros and build scripts, or if the `--target` flag isn't passed.
    Host,

    /// Attached to a unit to be compiled for a particular target. This is used
    /// for units when the `--target` flag is passed.
    Target(CompileTarget),
}

/// Fallback behavior in the
/// [`CompileKind::from_requested_targets_with_fallback`] function when
/// no targets are specified.
pub enum CompileKindFallback {
    /// The build configuration is consulted to find the default target, such as
    /// `$CARGO_BUILD_TARGET` or reading `build.target`.
    BuildConfig,

    /// Only the host should be returned when targets aren't explicitly
    /// specified. This is used by `cargo metadata` for example where "only
    /// host" has a special meaning in terms of the returned metadata.
    JustHost,
}

impl CompileKind {
    pub fn is_host(&self) -> bool {
        matches!(self, CompileKind::Host)
    }

    pub fn for_target(self, target: &Target) -> CompileKind {
        // Once we start compiling for the `Host` kind we continue doing so, but
        // if we are a `Target` kind and then we start compiling for a target
        // that needs to be on the host we lift ourselves up to `Host`.
        match self {
            CompileKind::Host => CompileKind::Host,
            CompileKind::Target(_) if target.for_host() => CompileKind::Host,
            CompileKind::Target(n) => CompileKind::Target(n),
        }
    }

    /// Creates a new list of `CompileKind` based on the requested list of
    /// targets.
    ///
    /// If no targets are given then this returns a single-element vector with
    /// `CompileKind::Host`.
    pub fn from_requested_targets(
        gctx: &GlobalContext,
        targets: &[String],
    ) -> CargoResult<Vec<CompileKind>> {
        CompileKind::from_requested_targets_with_fallback(
            gctx,
            targets,
            CompileKindFallback::BuildConfig,
        )
    }

    /// Same as [`CompileKind::from_requested_targets`] except that if `targets`
    /// doesn't explicitly mention anything the behavior of what to return is
    /// controlled by the `fallback` argument.
    pub fn from_requested_targets_with_fallback(
        gctx: &GlobalContext,
        targets: &[String],
        fallback: CompileKindFallback,
    ) -> CargoResult<Vec<CompileKind>> {
        let dedup = |targets: &[String]| {
            let deduplicated_targets = targets
                .iter()
                .map(|value| {
                    // This neatly substitutes the manually-specified `host-tuple` target directive
                    // with the compiling machine's target tuple.
                    if value.as_str() == "host-tuple" {
                        let host_triple = env!("RUST_HOST_TARGET");
                        Ok(CompileKind::Target(CompileTarget::new(
                            host_triple,
                            gctx.cli_unstable().json_target_spec,
                        )?))
                    } else {
                        Ok(CompileKind::Target(CompileTarget::new(
                            value.as_str(),
                            gctx.cli_unstable().json_target_spec,
                        )?))
                    }
                })
                // First collect into a set to deduplicate any `--target` passed
                // more than once...
                .collect::<CargoResult<BTreeSet<_>>>()?
                // ... then generate a flat list for everything else to use.
                .into_iter()
                .collect();

            Ok(deduplicated_targets)
        };

        if !targets.is_empty() {
            return dedup(targets);
        }

        let kinds = match (fallback, &gctx.build_config()?.target) {
            (_, None) | (CompileKindFallback::JustHost, _) => Ok(vec![CompileKind::Host]),
            (CompileKindFallback::BuildConfig, Some(build_target_config)) => {
                dedup(&build_target_config.values(gctx.cwd())?)
            }
        };

        kinds
    }

    /// Hash used for fingerprinting.
    ///
    /// Metadata hashing uses the normal Hash trait, which does not
    /// differentiate on `.json` file contents. The fingerprint hash does
    /// check the contents.
    pub fn fingerprint_hash(&self) -> u64 {
        match self {
            CompileKind::Host => 0,
            CompileKind::Target(target) => compile_target_fingerprint_hash(target),
        }
    }

    /// Adds the `--target` flag to the given [`ProcessBuilder`] if this is a
    /// non-host build.
    pub fn add_target_arg(&self, builder: &mut ProcessBuilder) {
        if let CompileKind::Target(target) = self {
            builder.arg("--target").arg(target.rustc_target());
            if matches!(target, CompileTarget::Json { .. }) {
                builder.arg("-Zunstable-options");
            }
        }
    }
}

impl serde::ser::Serialize for CompileKind {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        match self {
            CompileKind::Host => None::<&str>.serialize(s),
            CompileKind::Target(t) => Some(t.rustc_target()).serialize(s),
        }
    }
}

/// Resolution of an [`ArtifactTarget`] to the [`CompileKind`] it should be
/// built for.
///
/// This lives with the compiler rather than on `ArtifactTarget` itself because
/// the answer depends on compiler concepts (the root unit's compile kind) that
/// the dependency model is otherwise unaware of.
pub trait ArtifactTargetExt {
    /// The compile kind this target forces, if any.
    fn to_compile_kind(&self) -> Option<CompileKind>;

    /// The compile kind to build for, given the compile kind of the root unit.
    fn to_resolved_compile_kind(&self, root_unit_compile_kind: CompileKind) -> CompileKind;

    /// Like [`ArtifactTargetExt::to_resolved_compile_kind`], but `None` when the
    /// result is the host.
    fn to_resolved_compile_target(
        &self,
        root_unit_compile_kind: CompileKind,
    ) -> Option<CompileTarget>;
}

impl ArtifactTargetExt for ArtifactTarget {
    fn to_compile_kind(&self) -> Option<CompileKind> {
        self.to_compile_target().map(CompileKind::Target)
    }

    fn to_resolved_compile_kind(&self, root_unit_compile_kind: CompileKind) -> CompileKind {
        match self {
            ArtifactTarget::Force(target) => CompileKind::Target(*target),
            ArtifactTarget::BuildDependencyAssumeTarget => root_unit_compile_kind,
        }
    }

    fn to_resolved_compile_target(
        &self,
        root_unit_compile_kind: CompileKind,
    ) -> Option<CompileTarget> {
        match self.to_resolved_compile_kind(root_unit_compile_kind) {
            CompileKind::Host => None,
            CompileKind::Target(target) => Some(target),
        }
    }
}

/// Hash of a [`CompileTarget`] for fingerprinting; for JSON targets this
/// covers the file's contents, so edits to the spec invalidate builds.
fn compile_target_fingerprint_hash(target: &CompileTarget) -> u64 {
    let mut hasher = StableHasher::new();
    match target {
        CompileTarget::Tuple(name) => name.hash(&mut hasher),
        CompileTarget::Json { path, .. } => {
            // Hash the contents of the json file so that if it changes, the
            // fingerprint changes too.
            match fs::read_to_string(path) {
                Ok(contents) => contents.hash(&mut hasher),
                Err(_) => path.hash(&mut hasher),
            }
        }
    }
    Hasher::finish(&hasher)
}
