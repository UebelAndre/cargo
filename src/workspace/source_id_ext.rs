//! Cargo-specific operations on [`SourceId`].
//!
//! The [`SourceId`] type itself lives in [`cargo_resolver`]. The operations
//! here need Cargo's configuration or its [`Source`] implementations, so they
//! are provided as an extension trait.

use std::path::Path;

use tracing::trace;

use super::SourceId;

use crate::context;
use crate::sources::registry::CRATES_IO_HTTP_INDEX;
use crate::sources::source::Source;
use crate::sources::{CRATES_IO_REGISTRY, DirectorySource};
use crate::sources::{GitSource, PathSource, RegistrySource};
use crate::util::{CargoResult, GlobalContext, IntoUrl};
use crate::workspace::SourceKind;

/// Operations on [`SourceId`] that depend on Cargo's configuration or sources.
///
/// Import this trait to call these as if they were inherent methods, e.g.
/// `SourceId::crates_io(gctx)`.
pub trait SourceIdExt {
    /// Creates a `SourceId` from a manifest path.
    ///
    /// Single-file packages use the file itself as the source; everything
    /// else uses the containing directory.
    fn for_manifest_path(manifest_path: &Path) -> CargoResult<SourceId>;

    /// Returns the `SourceId` corresponding to the main repository.
    ///
    /// This is the main cargo registry by default, but it can be overridden in
    /// a `.cargo/config.toml`.
    fn crates_io(gctx: &GlobalContext) -> CargoResult<SourceId>;

    /// Returns the `SourceId` corresponding to the main repository, using the
    /// sparse HTTP index if allowed.
    fn crates_io_maybe_sparse_http(gctx: &GlobalContext) -> CargoResult<SourceId>;

    /// Returns whether to access crates.io over the sparse protocol.
    fn crates_io_is_sparse(gctx: &GlobalContext) -> CargoResult<bool>;

    /// Gets the `SourceId` associated with given name of the remote registry.
    fn alt_registry(gctx: &GlobalContext, key: &str) -> CargoResult<SourceId>;

    /// Creates an implementation of `Source` corresponding to this ID.
    fn load<'a>(self, gctx: &'a GlobalContext) -> CargoResult<Box<dyn Source + 'a>>;
}

impl SourceIdExt for SourceId {
    fn for_manifest_path(manifest_path: &Path) -> CargoResult<SourceId> {
        if crate::workspace::parser::is_embedded(manifest_path) && manifest_path.is_file() {
            SourceId::for_path(manifest_path)
        } else {
            SourceId::for_path(manifest_path.parent().unwrap())
        }
    }

    fn crates_io(gctx: &GlobalContext) -> CargoResult<SourceId> {
        gctx.crates_io_source_id()
    }

    fn crates_io_maybe_sparse_http(gctx: &GlobalContext) -> CargoResult<SourceId> {
        if Self::crates_io_is_sparse(gctx)? {
            gctx.check_registry_index_not_set()?;
            let url = CRATES_IO_HTTP_INDEX.into_url().unwrap();
            SourceId::for_alt_registry(&url, CRATES_IO_REGISTRY)
        } else {
            Self::crates_io(gctx)
        }
    }

    fn crates_io_is_sparse(gctx: &GlobalContext) -> CargoResult<bool> {
        let proto: Option<context::Value<String>> =
            gctx.get(["registries", "crates-io", "protocol"])?;
        let is_sparse = match proto.as_ref().map(|v| v.val.as_str()) {
            Some("sparse") => true,
            Some("git") => false,
            Some(unknown) => anyhow::bail!(
                "unsupported registry protocol `{unknown}` (defined in {})",
                proto.as_ref().unwrap().definition
            ),
            None => true,
        };
        Ok(is_sparse)
    }

    fn alt_registry(gctx: &GlobalContext, key: &str) -> CargoResult<SourceId> {
        if key == CRATES_IO_REGISTRY {
            return Self::crates_io(gctx);
        }
        let url = gctx.get_registry_index(key)?;
        Self::for_alt_registry(&url, key)
    }

    fn load<'a>(self, gctx: &'a GlobalContext) -> CargoResult<Box<dyn Source + 'a>> {
        trace!("loading SourceId; {}", self);
        match self.kind() {
            SourceKind::Git(..) => Ok(Box::new(GitSource::new(self, gctx)?)),
            SourceKind::Path => {
                let path = self
                    .url()
                    .to_file_path()
                    .expect("path sources cannot be remote");
                if crate::workspace::parser::is_embedded(&path) && path.is_file() {
                    anyhow::bail!("single file packages cannot be used as dependencies")
                }
                Ok(Box::new(PathSource::new(&path, self, gctx)))
            }
            SourceKind::Builtin => todo!("builtin source"),
            SourceKind::Registry | SourceKind::SparseRegistry => {
                Ok(Box::new(RegistrySource::remote(self, gctx)?))
            }
            SourceKind::LocalRegistry => {
                let path = self
                    .url()
                    .to_file_path()
                    .expect("path sources cannot be remote");
                Ok(Box::new(RegistrySource::local(self, &path, gctx)))
            }
            SourceKind::Directory => {
                let path = self
                    .url()
                    .to_file_path()
                    .expect("path sources cannot be remote");
                Ok(Box::new(DirectorySource::new(&path, self, gctx)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SourceId, SourceIdExt};
    use crate::util::{GlobalContext, IntoUrl};
    use crate::workspace::GitReference;

    // This is a test that the hash of the `SourceId` for crates.io is a well-known
    // value.
    //
    // Note that the hash value matches what the crates.io source id has hashed
    // since Rust 1.84.0. We strive to keep this value the same across
    // versions of Cargo because changing it means that users will need to
    // redownload the index and all crates they use when using a new Cargo version.
    //
    // This isn't to say that this hash can *never* change, only that when changing
    // this it should be explicitly done. If this hash changes accidentally and
    // you're able to restore the hash to its original value, please do so!
    // Otherwise please just leave a comment in your PR as to why the hash value is
    // changing and why the old value can't be easily preserved.
    // If it takes an ugly hack to restore it,
    // then leave a link here so we can remove the hack next time we change the hash.
    //
    // Hacks to remove next time the hash changes:
    // - (fill in your code here)
    //
    // The hash value should be stable across platforms, and doesn't depend on
    // endianness and bit-width. One caveat is that absolute paths on Windows
    // are inherently different than on Unix-like platforms. Unless we omit or
    // strip the prefix components (e.g. `C:`), there is not way to have a true
    // cross-platform stable hash for absolute paths.
    #[test]
    fn test_stable_hash() {
        use std::hash::Hasher;
        use std::path::Path;

        use snapbox::IntoData as _;
        use snapbox::assert_data_eq;
        use snapbox::str;

        use crate::util::StableHasher;
        use crate::util::hex::short_hash;

        #[cfg(not(windows))]
        let ws_root = Path::new("/tmp/ws");
        #[cfg(windows)]
        let ws_root = Path::new(r"C:\\tmp\ws");

        let gen_hash = |source_id: SourceId| {
            let mut hasher = StableHasher::new();
            source_id.stable_hash(ws_root, &mut hasher);
            Hasher::finish(&hasher).to_string()
        };

        let source_id = SourceId::crates_io(&GlobalContext::default().unwrap()).unwrap();
        assert_data_eq!(gen_hash(source_id), str!["7062945687441624357"].raw());
        assert_data_eq!(short_hash(&source_id), str!["25cdd57fae9f0462"].raw());

        let url = "https://my-crates.io".into_url().unwrap();
        let source_id = SourceId::for_registry(&url).unwrap();
        assert_data_eq!(gen_hash(source_id), str!["8310250053664888498"].raw());
        assert_data_eq!(short_hash(&source_id), str!["b2d65deb64f05373"].raw());

        let url = "https://your-crates.io".into_url().unwrap();
        let source_id = SourceId::for_alt_registry(&url, "alt").unwrap();
        assert_data_eq!(gen_hash(source_id), str!["14149534903000258933"].raw());
        assert_data_eq!(short_hash(&source_id), str!["755952de063f5dc4"].raw());

        let url = "sparse+https://my-crates.io".into_url().unwrap();
        let source_id = SourceId::for_registry(&url).unwrap();
        assert_data_eq!(gen_hash(source_id), str!["16249512552851930162"].raw());
        assert_data_eq!(short_hash(&source_id), str!["327cfdbd92dd81e1"].raw());

        let url = "sparse+https://your-crates.io".into_url().unwrap();
        let source_id = SourceId::for_alt_registry(&url, "alt").unwrap();
        assert_data_eq!(gen_hash(source_id), str!["6156697384053352292"].raw());
        assert_data_eq!(short_hash(&source_id), str!["64a713b6a6fb7055"].raw());

        let url = "file:///tmp/ws/crate".into_url().unwrap();
        let source_id = SourceId::for_git(&url, GitReference::DefaultBranch).unwrap();
        assert_data_eq!(gen_hash(source_id), str!["473480029881867801"].raw());
        assert_data_eq!(short_hash(&source_id), str!["199e591d94239206"].raw());

        let source_id = SourceId::for_builtin().unwrap();
        assert_data_eq!(gen_hash(source_id), str!["5062137541645472653"].raw());
        assert_data_eq!(short_hash(&source_id), str!["8d93f21546534046"].raw());

        let path = &ws_root.join("crate");
        let source_id = SourceId::for_local_registry(path).unwrap();
        #[cfg(not(windows))]
        {
            assert_data_eq!(gen_hash(source_id), str!["11515846423845066584"].raw());
            assert_data_eq!(short_hash(&source_id), str!["58d73c154f81d09f"].raw());
        }
        #[cfg(windows)]
        {
            assert_data_eq!(gen_hash(source_id), str!["6146331155906064276"].raw());
            assert_data_eq!(short_hash(&source_id), str!["946fb2239f274c55"].raw());
        }

        let source_id = SourceId::for_path(path).unwrap();
        assert_data_eq!(gen_hash(source_id), str!["215644081443634269"].raw());
        #[cfg(not(windows))]
        assert_data_eq!(short_hash(&source_id), str!["64bace89c92b101f"].raw());
        #[cfg(windows)]
        assert_data_eq!(short_hash(&source_id), str!["01e1e6c391813fb6"].raw());

        let source_id = SourceId::for_directory(path).unwrap();
        #[cfg(not(windows))]
        {
            assert_data_eq!(gen_hash(source_id), str!["6127590343904940368"].raw());
            assert_data_eq!(short_hash(&source_id), str!["505191d1f3920955"].raw());
        }
        #[cfg(windows)]
        {
            assert_data_eq!(gen_hash(source_id), str!["10423446877655960172"].raw());
            assert_data_eq!(short_hash(&source_id), str!["6c8ad69db585a790"].raw());
        }
    }
}
