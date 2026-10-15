//! Conversions between filesystem paths and `file://` URLs.
//!
//! The `url` crate only provides these conversions on targets whose path
//! syntax it knows about. Elsewhere, such as `wasm32-unknown-unknown`, paths
//! are treated as absolute, unix-style, UTF-8 paths.

use std::path::Path;
use std::path::PathBuf;

use url::Url;

#[cfg(any(
    unix,
    windows,
    target_os = "redox",
    target_os = "wasi",
    target_os = "hermit"
))]
pub(crate) fn from_file_path(path: &Path) -> Result<Url, ()> {
    Url::from_file_path(path)
}

#[cfg(any(
    unix,
    windows,
    target_os = "redox",
    target_os = "wasi",
    target_os = "hermit"
))]
pub(crate) fn to_file_path(url: &Url) -> Result<PathBuf, ()> {
    url.to_file_path()
}

#[cfg(not(any(
    unix,
    windows,
    target_os = "redox",
    target_os = "wasi",
    target_os = "hermit"
)))]
pub(crate) fn from_file_path(path: &Path) -> Result<Url, ()> {
    use std::path::Component;

    if !path.is_absolute() {
        return Err(());
    }
    let mut url = Url::parse("file:///").unwrap();
    {
        let mut segments = url.path_segments_mut()?;
        segments.clear();
        for component in path.components() {
            match component {
                Component::RootDir => {}
                Component::Prefix(_) => return Err(()),
                component => {
                    segments.push(component.as_os_str().to_str().ok_or(())?);
                }
            }
        }
    }
    Ok(url)
}

#[cfg(not(any(
    unix,
    windows,
    target_os = "redox",
    target_os = "wasi",
    target_os = "hermit"
)))]
pub(crate) fn to_file_path(url: &Url) -> Result<PathBuf, ()> {
    if url.scheme() != "file" {
        return Err(());
    }
    match url.host_str() {
        None | Some("") | Some("localhost") => {}
        Some(_) => return Err(()),
    }
    let mut path = String::new();
    for segment in url.path_segments().ok_or(())? {
        path.push('/');
        let decoded = percent_encoding::percent_decode_str(segment)
            .decode_utf8()
            .map_err(|_| ())?;
        path.push_str(&decoded);
    }
    Ok(PathBuf::from(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let path = Path::new("/tmp/some dir/crate");
        let url = from_file_path(path).unwrap();
        assert_eq!(url.as_str(), "file:///tmp/some%20dir/crate");
        assert_eq!(to_file_path(&url).unwrap(), path);
    }

    #[test]
    fn rejects_relative() {
        assert!(from_file_path(Path::new("relative/path")).is_err());
    }

    #[test]
    fn rejects_non_file() {
        let url = Url::parse("https://example.com/a").unwrap();
        assert!(to_file_path(&url).is_err());
    }
}
