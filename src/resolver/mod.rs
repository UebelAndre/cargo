//! Cargo's integration with the dependency resolver.
//!
//! The resolver itself lives in the [`cargo_resolver`] crate. This module
//! re-exports it, adapts [`GlobalContext`] to its [`ResolverHost`], reads the
//! configuration it needs, and hosts the feature resolver in [`features`].

use crate::context::GlobalContext;
use crate::context::{
    CargoResolverConfig, GlobalRegistryConfig, IncompatiblePublishAge, RegistryConfig,
};
use crate::util::data_structures::HashMap;
use crate::util::errors::CargoResult;
use crate::util::time_span::parse_time_span;
use crate::workspace::{Dependency, PackageIdSpec, Registry, Summary};

pub use cargo_resolver::encode;
pub use cargo_resolver::errors;
pub use cargo_resolver::{
    ActivateError, ActivateResult, MinPublishAge, PublishAgePolicy, PublishAgeViolation, Resolve,
    ResolveBehavior, ResolveError, ResolveOpts, ResolveVersion, ResolverHost, VersionOrdering,
    VersionPreferences,
};
// Kept for `resolver-tests` until it depends on `cargo-resolver` directly.
pub use cargo_resolver::resolve as resolve_with_host;

pub use self::features::{CliFeatures, ForceAllTargets, HasDevUnits};
pub use self::global_context_host::GlobalContextHost;

pub mod features;
mod global_context_host;

/// Builds the list of all packages required to build the first argument.
///
/// * `summaries` - the list of package summaries along with how to resolve
///   their features. This is a list of all top-level packages that are intended
///   to be part of the lock file (resolve output). These typically are a list
///   of all workspace members.
///
/// * `replacements` - this is a list of `[replace]` directives found in the
///   root of the workspace. The list here is a `PackageIdSpec` of what to
///   replace and a `Dependency` to replace that with. In general it's not
///   recommended to use `[replace]` any more and use `[patch]` instead, which
///   is supported elsewhere.
///
/// * `registry` - this is the source from which all package summaries are
///   loaded. It's expected that this is extensively configured ahead of time
///   and is idempotent with our requests to it (aka returns the same results
///   for the same query every time). Typically this is an instance of a
///   `PackageRegistry`.
///
/// * `version_prefs` - this represents a preference for some versions over others,
///   based on the lock file or other reasons such as `[patch]`es.
///
/// * `resolve_version` - this controls how the lockfile will be serialized.
///
/// * `gctx` - the global context used for configuration and diagnostics.
#[tracing::instrument(skip_all)]
pub fn resolve(
    summaries: &[(Summary, ResolveOpts)],
    replacements: &[(PackageIdSpec, Dependency)],
    registry: &impl Registry,
    version_prefs: &VersionPreferences,
    resolve_version: ResolveVersion,
    gctx: &GlobalContext,
) -> CargoResult<Resolve> {
    let host = GlobalContextHost::new(gctx);
    cargo_resolver::resolve(
        summaries,
        replacements,
        registry,
        version_prefs,
        resolve_version,
        &host,
    )
}

/// Builds the [`PublishAgePolicy`] from `min-publish-age` configuration.
///
/// Returns `None` when either
///
/// * the resolver is configured to allow pubtime-incompatible versions
/// * no threshold is configured at all
pub fn publish_age_policy(
    now: Option<jiff::Timestamp>,
    gctx: &GlobalContext,
) -> CargoResult<Option<PublishAgePolicy>> {
    let resolver_config = gctx.get::<Option<CargoResolverConfig>>(["resolver"])?;
    if resolver_config
        .and_then(|c| c.incompatible_publish_age)
        .is_some_and(|v| v == IncompatiblePublishAge::Allow)
    {
        return Ok(None);
    }
    publish_age_policy_for_report(now, gctx)
}

/// Like [`publish_age_policy`] but ignores config from `[resolver]`, so it
/// reports too-new packages regardless of whether they are allowed or denied.
pub fn publish_age_policy_for_report(
    now: Option<jiff::Timestamp>,
    gctx: &GlobalContext,
) -> CargoResult<Option<PublishAgePolicy>> {
    let (global, per_registry) = read_min_publish_age(gctx)?;
    let invocation_time = now.unwrap_or_else(|| gctx.invocation_time());
    Ok(PublishAgePolicy::from_parts(
        invocation_time,
        global,
        per_registry,
    ))
}

/// Reads `registry.global-min-publish-age` and every
/// `registries.<name>.min-publish-age` from configuration.
fn read_min_publish_age(
    gctx: &GlobalContext,
) -> CargoResult<(MinPublishAge, HashMap<String, MinPublishAge>)> {
    let parse = |key: &str, config: Option<String>| -> CargoResult<MinPublishAge> {
        let Some(config) = config else {
            return Ok(MinPublishAge::Unset);
        };
        if config == "0" {
            return Ok(MinPublishAge::None);
        }
        let duration = parse_time_span(&config)
            .map_err(|e| anyhow::format_err!("invalid value for `{key}`: {e}"))?;
        Ok(MinPublishAge::Age(duration, config))
    };

    let registry = gctx.get::<Option<GlobalRegistryConfig>>(["registry"])?;
    let global = parse(
        "registry.global-min-publish-age",
        registry.and_then(|r| r.global_min_publish_age),
    )?;
    let mut per_registry = HashMap::default();
    if let Some(registries) = gctx.get::<Option<HashMap<String, RegistryConfig>>>(["registries"])? {
        for (name, config) in registries {
            let limit = parse(
                &format!("registries.{name}.min-publish-age"),
                config.min_publish_age,
            )?;
            per_registry.insert(name, limit);
        }
    }
    Ok((global, per_registry))
}
