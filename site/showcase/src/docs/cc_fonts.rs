use openui::prelude::*;

const GEIST_MONO: &[u8] =
    include_bytes!("../../../../components/assets/fonts/geist/GeistMono-Variable.woff2");

pub fn example() -> ViewNode {
    // Register the bytes under a family name. A variable font declares its
    // weight range, so every weight in it can be used.
    let mut mono = FontFaceDescriptor::new("Geist Mono");
    mono.weight = FontAxisRange::new(100.0, 900.0);
    current_document()
        .register_font_face(GEIST_MONO, mono)
        .expect("register Geist Mono");
    view! {
        <div style:line-height="1.4">
            <div style:font-size="28px" style:font-weight="700" style:letter-spacing="-1px">
                "Release 0.2"
            </div>
            <div style:font-size="15px" style:font-weight="400" style:color="#52525b">
                "Geist, at weights 700 and 400."
            </div>
            <div
                style:margin-top="10px"
                style:font-family="Geist Mono"
                style:font-size="13px"
                style:padding="8px 10px"
                style:background-color="#f4f4f5"
                style:border-radius="6px"
            >
                "cargo install --locked openui-cli"
            </div>
        </div>
    }
}
