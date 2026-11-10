//! The bundled fonts: Geist, Geist Mono and Inter.
//!
//! The files in `components/assets/fonts` are unmodified copies from the
//! npm packages pinned by `components/reference/package-lock.json` (Geist
//! from `geist`, Inter from `inter-ui`; both SIL OFL 1.1, license files
//! beside them). The reference pages load the same files with the same
//! `@font-face` descriptors, listed in `components/assets/manifest.json`.
//! shadcn/ui's site uses Geist as its sans-serif face and Geist Mono as its
//! monospace face.

use openui::{Document, Error, FontAxisRange, FontFaceDescriptor, FontFaceHandle, FontStyleRange};

/// One bundled face: its CSS family name and its file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BundledFont {
    /// The `font-family` name it is registered under.
    pub family: &'static str,
    /// Its path under `components/assets`.
    pub file: &'static str,
    /// The WOFF2 bytes.
    pub bytes: &'static [u8],
}

/// Geist, shadcn/ui's sans-serif face (`--font-sans`).
pub const GEIST: BundledFont = BundledFont {
    family: "Geist",
    file: "fonts/geist/Geist-Variable.woff2",
    bytes: include_bytes!("../../assets/fonts/geist/Geist-Variable.woff2"),
};

/// Geist Mono, shadcn/ui's monospace face (`--font-mono`).
pub const GEIST_MONO: BundledFont = BundledFont {
    family: "Geist Mono",
    file: "fonts/geist/GeistMono-Variable.woff2",
    bytes: include_bytes!("../../assets/fonts/geist/GeistMono-Variable.woff2"),
};

/// Inter, the alternative sans-serif face (`--font-inter`).
pub const INTER: BundledFont = BundledFont {
    family: "Inter",
    file: "fonts/inter/InterVariable.woff2",
    bytes: include_bytes!("../../assets/fonts/inter/InterVariable.woff2"),
};

/// Every bundled face, in the order of the assets manifest.
pub const BUNDLED_FONTS: [BundledFont; 3] = [GEIST, GEIST_MONO, INTER];

/// The CSS family list for shadcn's sans-serif text.
pub const FONT_SANS: &str = "Geist";
/// The CSS family list for shadcn's monospace text.
pub const FONT_MONO: &str = "Geist Mono";

impl BundledFont {
    /// The descriptor the reference pages' `@font-face` rule declares: a
    /// variable face covering weights 100 to 900, normal style.
    pub fn descriptor(&self) -> FontFaceDescriptor {
        let mut descriptor = FontFaceDescriptor::new(self.family);
        descriptor.style = FontStyleRange::Normal;
        descriptor.weight = FontAxisRange::new(100.0, 900.0);
        descriptor
    }
}

/// Register every bundled face with `document`.
pub fn register_fonts(document: &Document) -> Result<Vec<FontFaceHandle>, Error> {
    BUNDLED_FONTS
        .iter()
        .map(|font| document.register_font_face(font.bytes, font.descriptor()))
        .collect()
}
