use crate::data::ModLoader;
use bytes::Bytes;

/// Returns the built-in icon bytes for a given mod loader.
/// Returns `None` for Vanilla (no default icon).
pub fn get_builtin_icon_bytes(
    loader: &ModLoader,
) -> Option<(&'static str, Bytes)> {
    match loader {
        ModLoader::Fabric => Some((
            "fabric.png",
            Bytes::from_static(
                include_bytes!("assets/icons/fabric.png").as_slice(),
            ),
        )),
        ModLoader::Forge => Some((
            "anvil.png",
            Bytes::from_static(
                include_bytes!("assets/icons/anvil.png").as_slice(),
            ),
        )),
        ModLoader::NeoForge => Some((
            "neoforge.png",
            Bytes::from_static(
                include_bytes!("assets/icons/neoforge.png").as_slice(),
            ),
        )),
        ModLoader::Quilt => Some((
            "quilt.png",
            Bytes::from_static(
                include_bytes!("assets/icons/quilt.png").as_slice(),
            ),
        )),
        ModLoader::Vanilla | ModLoader::OptiFine => None,
    }
}
