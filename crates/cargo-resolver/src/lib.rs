//! Cargo's dependency resolver.
//!
//! > This crate is maintained by the Cargo team, primarily for use by Cargo
//! > and not intended for external use (except as a transitive dependency). This
//! > crate may make major changes to its APIs or be deprecated without warning.
//!
//! This crate contains the algorithm Cargo uses to turn a set of root packages
//! and their dependency requirements into a locked dependency graph, along with
//! the types that describe that graph.
//!
//! The entry point is [`resolve`]. It takes:
//!
//! - the [`Summary`] of each root package, with [`ResolveOpts`] saying which
//!   features and dependency kinds to consider,
//! - a [`Registry`], which the resolver queries for the candidates satisfying
//!   each [`Dependency`],
//! - [`VersionPreferences`], which steer which candidates are tried first
//!   (e.g. versions from an existing lock file), and
//! - a [`ResolverHost`], through which the resolver reaches back into the
//!   embedding application for the handful of things it cannot decide itself.
//!
//! It produces a [`Resolve`], the dependency graph with every package pinned
//! to an exact [`PackageId`]. [`encode`] converts between a [`Resolve`] and
//! the `Cargo.lock` representation.
//!
//! ## Portability
//!
//! The resolver performs no I/O, reads no environment, and does not depend on
//! a clock. Everything of that kind is provided by the host through
//! [`ResolverHost`] and the [`Registry`] implementation. The crate therefore
//! builds for targets such as `wasm32-unknown-unknown`.
//!
//! ## Memory
//!
//! Package, feature, dependency, and registry names are [`InternedString`]s,
//! which live in a process-global table for the lifetime of the process, as
//! they do in Cargo. Each distinct string is interned once: resolving a large
//! workspace interns a few thousand short strings, and resolving the same
//! inputs again interns nothing new. This crate is designed for Cargo's own
//! usage pattern, a process that resolves and exits. A long-running process
//! that keeps resolving new inputs will see the table grow with the number of
//! distinct names it encounters.
//!
//! ## Errors
//!
//! Functions in this crate return [`anyhow::Error`] through the
//! [`CargoResult`] alias. Resolution failures are reported as [`ResolveError`],
//! which carries the chain of packages that led to the failing requirement.

pub use cargo_util_schemas::core::GitReference;
pub use cargo_util_schemas::core::PackageIdSpec;
pub use cargo_util_schemas::core::SourceKind;

pub use self::canonical_url::CanonicalUrl;
pub use self::compile_target::CompileTarget;
pub use self::dependency::Artifact;
pub use self::dependency::ArtifactKind;
pub use self::dependency::ArtifactTarget;
pub use self::dependency::DepKind;
pub use self::dependency::Dependency;
pub use self::dependency::SerializedDependency;
pub use self::graph::Graph;
pub use self::host::ResolverHost;
pub use self::interning::InternedString;
pub use self::into_url::IntoUrl;
pub use self::package_id::PackageId;
pub use self::package_id::PackageIdStableHash;
pub use self::package_id_spec::PackageIdSpecQuery;
pub use self::registry::CRATES_IO_DOMAIN;
pub use self::registry::CRATES_IO_HTTP_INDEX;
pub use self::registry::CRATES_IO_INDEX;
pub use self::registry::CRATES_IO_REGISTRY;
pub use self::registry::IndexSummary;
pub use self::registry::QueryKind;
pub use self::registry::Registry;
pub use self::resolver::ActivateError;
pub use self::resolver::ActivateResult;
pub use self::resolver::CliFeatures;
pub use self::resolver::FeaturesSet;
pub use self::resolver::MinPublishAge;
pub use self::resolver::PublishAgePolicy;
pub use self::resolver::PublishAgeViolation;
pub use self::resolver::RequestedFeatures;
pub use self::resolver::Resolve;
pub use self::resolver::ResolveBehavior;
pub use self::resolver::ResolveError;
pub use self::resolver::ResolveOpts;
pub use self::resolver::ResolveVersion;
pub use self::resolver::VersionOrdering;
pub use self::resolver::VersionPreferences;
pub use self::resolver::encode;
pub use self::resolver::errors;
pub use self::resolver::resolve;
pub use self::semver_ext::OptVersionReq;
pub use self::semver_ext::VersionExt;
pub use self::semver_ext::VersionReqExt;
pub use self::semver_ext::VersionReqMatchMode;
pub use self::source_id::SourceId;
pub use self::source_id::SourceIdAsUrl;
pub use self::summary::FeatureMap;
pub use self::summary::FeatureValue;
pub use self::summary::MissingDependencyError;
pub use self::summary::Summary;

mod canonical_url;
mod compile_target;
mod data_structures;
mod dependency;
pub mod edit_distance;
mod file_url;
mod graph;
mod host;
pub mod interning;
mod into_url;
mod local_poll_adapter;
mod package_id;
mod package_id_spec;
mod registry;
mod resolver;
mod semver_eval_ext;
mod semver_ext;
mod source_id;
mod summary;

/// The result type used throughout this crate.
///
/// This is the same alias Cargo uses, so values pass between the two without
/// conversion.
pub type CargoResult<T> = anyhow::Result<T>;
