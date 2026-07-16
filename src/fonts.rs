use ab_glyph::{FontRef, PxScale};

/// Embedded default font (DejaVu Sans Mono).
///
/// The font data is embedded at compile time via `include_bytes!` and
/// parsed on each call (cheap — no heap allocation).
pub(crate) fn default_font() -> FontRef<'static> {
    FontRef::try_from_slice(include_bytes!("../fonts/DejaVuSansMono.ttf"))
        .expect("failed to load embedded font")
}

/// Scale the font to the given pixel size.
pub(crate) fn scale(size: f32) -> PxScale {
    PxScale::from(size)
}
