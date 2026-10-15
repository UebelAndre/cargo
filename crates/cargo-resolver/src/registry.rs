//! The [`Registry`] trait the dependency resolver queries, and the types it
//! is expressed in.
//!
//! Cargo's `PackageRegistry` is one implementation; `resolver-tests` has another.

use crate::CargoResult;
use crate::{Dependency, PackageId, SourceId, Summary};

pub const CRATES_IO_INDEX: &str = "https://github.com/rust-lang/crates.io-index";
pub const CRATES_IO_HTTP_INDEX: &str = "sparse+https://index.crates.io/";
pub const CRATES_IO_REGISTRY: &str = "crates-io";
pub const CRATES_IO_DOMAIN: &str = "crates.io";

/// Defines how a dependency query will be performed for a `Source`.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum QueryKind {
    /// A query for packages exactly matching the given dependency requirement.
    ///
    /// Each source gets to define what `exact` means for it.
    Exact,
    /// A query for packages close to the given dependency requirement.
    ///
    /// Each source gets to define what `close` means for it.
    ///
    /// Path/Git sources may return all dependencies that are at that URI,
    /// whereas an `Registry` source may return dependencies that are yanked or invalid.
    RejectedVersions,
    /// A query for packages close to the given dependency requirement.
    ///
    /// Each source gets to define what `close` means for it.
    ///
    /// Path/Git sources may return all dependencies that are at that URI,
    /// whereas an `Registry` source may return dependencies that have the same
    /// canonicalization.
    AlternativeNames,
    /// Match a dependency in all ways and will normalize the package name.
    /// Each source defines what normalizing means.
    Normalized,
}

/// A parsed representation of a summary from the index. This is usually parsed
/// from a line from a raw index file, or a JSON blob from on-disk index cache.
///
/// In addition to a full [`Summary`], we have information on whether it is `yanked`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum IndexSummary {
    /// Available for consideration
    Candidate(Summary),
    /// Yanked within its registry
    Yanked(Summary),
    /// Not available as we are offline and create is not downloaded yet
    Offline(Summary),
    /// From a newer schema version and is likely incomplete or inaccurate
    Unsupported(Summary, u32),
    /// An error was encountered despite being a supported schema version
    Invalid(Summary),
}

impl IndexSummary {
    /// Extract the summary from any variant.
    ///
    /// You should not use this unless you know what you are doing.
    pub fn as_summary_unchecked(&self) -> &Summary {
        match self {
            IndexSummary::Candidate(sum)
            | IndexSummary::Yanked(sum)
            | IndexSummary::Offline(sum)
            | IndexSummary::Unsupported(sum, _)
            | IndexSummary::Invalid(sum) => sum,
        }
    }

    pub fn map_summary(self, f: impl Fn(Summary) -> Summary) -> Self {
        match self {
            IndexSummary::Candidate(s) => IndexSummary::Candidate(f(s)),
            IndexSummary::Yanked(s) => IndexSummary::Yanked(f(s)),
            IndexSummary::Offline(s) => IndexSummary::Offline(f(s)),
            IndexSummary::Unsupported(s, v) => IndexSummary::Unsupported(f(s), v.clone()),
            IndexSummary::Invalid(s) => IndexSummary::Invalid(f(s)),
        }
    }

    /// Extract the package id from any variant
    pub fn package_id(&self) -> PackageId {
        self.as_summary_unchecked().package_id()
    }

    /// Returns `true` if the index summary is [`Yanked`].
    ///
    /// [`Yanked`]: IndexSummary::Yanked
    #[must_use]
    pub fn is_yanked(&self) -> bool {
        matches!(self, Self::Yanked(..))
    }

    /// Returns `true` if the index summary is [`Offline`].
    ///
    /// [`Offline`]: IndexSummary::Offline
    #[must_use]
    pub fn is_offline(&self) -> bool {
        matches!(self, Self::Offline(..))
    }
}

/// An abstraction provides a set of methods for querying source information
/// about a group of packages, without leaking too much implementation details
/// of the actual registry.
///
/// As of 2024-04, only Cargo's `PackageRegistry` and `MyRegistry` in resolver-tests
/// are found implementing this.
///
/// See also the `Source` trait, as many of the methods here mirror and
/// abstract over its functionalities.
#[allow(async_fn_in_trait)]
pub trait Registry {
    /// Attempt to find the packages that match a dependency request.
    async fn query(
        &self,
        dep: &Dependency,
        kind: QueryKind,
        f: &mut dyn FnMut(IndexSummary),
    ) -> CargoResult<()>;

    /// Gathers the result from [`Registry::query`] as a list of [`IndexSummary`] items
    /// when they become available.
    async fn query_vec(&self, dep: &Dependency, kind: QueryKind) -> CargoResult<Vec<IndexSummary>> {
        let mut ret = Vec::new();
        self.query(dep, kind, &mut |s| ret.push(s))
            .await
            .map(|()| ret)
    }

    /// Gets the description of a source, to provide useful messages.
    fn describe_source(&self, source: SourceId) -> String;

    /// Checks if a source is replaced with some other source.
    fn is_replaced(&self, source: SourceId) -> bool;
}
