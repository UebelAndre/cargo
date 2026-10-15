pub use self::dependency::{Dependency, SerializedDependency};
pub use self::features::{CliUnstable, Edition, Feature, Features};
pub use self::manifest::{EitherManifest, VirtualManifest};
pub use self::manifest::{Manifest, Target, TargetKind};
pub use self::package::{Package, PackageSet};
pub use self::patch::{Patch, PatchLocation};
pub use self::registry::Registry;
pub use self::source_id_ext::SourceIdExt;
pub use self::workspace::{
    MaybePackage, Workspace, WorkspaceConfig, WorkspaceRootConfig, find_workspace_root,
    find_workspace_root_with_membership_check, resolve_relative_path,
};
pub use cargo_resolver::PackageId;
pub use cargo_resolver::PackageIdSpecQuery;
pub use cargo_resolver::SourceId;
pub use cargo_resolver::{FeatureMap, FeatureValue, Summary};
pub use cargo_util_schemas::core::{GitReference, PackageIdSpec, SourceKind};

pub mod editor;
pub mod features;
pub mod gc;
pub mod global_cache_tracker;
pub mod manifest;
pub mod package;
pub mod parser;
mod patch;
pub mod profiles;
pub mod registry;
mod source_id_ext;
mod workspace;

/// Dependency declarations; see [`cargo_resolver`].
pub mod dependency {
    pub use cargo_resolver::Artifact;
    pub use cargo_resolver::ArtifactKind;
    pub use cargo_resolver::ArtifactTarget;
    pub use cargo_resolver::DepKind;
    pub use cargo_resolver::Dependency;
    pub use cargo_resolver::SerializedDependency;
}

/// Identifiers for packages; see [`cargo_resolver`].
pub mod package_id {
    pub use cargo_resolver::PackageId;
    pub use cargo_resolver::PackageIdStableHash;
}

/// Package summaries and feature declarations; see [`cargo_resolver`].
pub mod summary {
    pub use cargo_resolver::FeatureMap;
    pub use cargo_resolver::FeatureValue;
    pub use cargo_resolver::MissingDependencyError;
    pub use cargo_resolver::Summary;
}
