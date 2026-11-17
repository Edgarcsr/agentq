//! The app's own icons (React Symbols, MIT; see `assets/icons`), served
//! before the kit's Lucide set.

use gpui_kit::{AssetSource, Result, SharedString};
use std::borrow::Cow;

const ICONS: &[(&str, &[u8])] = &[
    (
        "icons/node.svg",
        include_bytes!("../../assets/icons/node.svg"),
    ),
    (
        "icons/npm.svg",
        include_bytes!("../../assets/icons/npm.svg"),
    ),
    (
        "icons/python.svg",
        include_bytes!("../../assets/icons/python.svg"),
    ),
    ("icons/go.svg", include_bytes!("../../assets/icons/go.svg")),
    (
        "icons/java.svg",
        include_bytes!("../../assets/icons/java.svg"),
    ),
    (
        "icons/rust.svg",
        include_bytes!("../../assets/icons/rust.svg"),
    ),
    (
        "icons/typescript.svg",
        include_bytes!("../../assets/icons/typescript.svg"),
    ),
    (
        "icons/react.svg",
        include_bytes!("../../assets/icons/react.svg"),
    ),
    (
        "icons/docker.svg",
        include_bytes!("../../assets/icons/docker.svg"),
    ),
    (
        "icons/php.svg",
        include_bytes!("../../assets/icons/php.svg"),
    ),
    (
        "icons/ruby.svg",
        include_bytes!("../../assets/icons/ruby.svg"),
    ),
    (
        "icons/kotlin.svg",
        include_bytes!("../../assets/icons/kotlin.svg"),
    ),
    (
        "icons/csharp.svg",
        include_bytes!("../../assets/icons/csharp.svg"),
    ),
    (
        "icons/swift.svg",
        include_bytes!("../../assets/icons/swift.svg"),
    ),
    (
        "icons/folder.svg",
        include_bytes!("../../assets/icons/folder.svg"),
    ),
];

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some((_, bytes)) = ICONS.iter().find(|(p, _)| *p == path) {
            return Ok(Some(Cow::Borrowed(bytes)));
        }
        gpui_kit::assets::AllAssets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        gpui_kit::assets::AllAssets.list(path)
    }
}
