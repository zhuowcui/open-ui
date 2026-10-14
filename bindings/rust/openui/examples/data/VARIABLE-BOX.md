# Variable font example fixture

`variabletest_box.ttf` is an unchanged copy of the Chromium WPT font also
used by the text crate's regression test. Bundling it here keeps the example
independent of the workspace layout when the `openui` crate is packaged.

- Copyright 2017 The Chromium Authors. All rights reserved.
- License: SIL Open Font License 1.1, reproduced in `LICENSE-OFL-1.1.txt`.
- SHA-256: `9270b7f6b2b8b34215a80c63a9c1348e704a0e3803ee49f2531bd62fdb5139c4`.

The app registers these bytes through `Document::register_font_face` and
changes the `UPWD` axis through `Element::set_font_variation_settings`.
