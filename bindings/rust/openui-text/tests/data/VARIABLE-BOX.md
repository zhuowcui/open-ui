# Variable font regression fixture

`variabletest_box.ttf` is copied without modification from the pinned Chromium
WPT checkout's `css/css-fonts/variations/resources/variabletest_box.ttf`.

- Copyright 2017 The Chromium Authors. All rights reserved.
- License: SIL Open Font License 1.1, reproduced in `../../fonts/LICENSE-OFL-1.1.txt`.
- SHA-256: `9270b7f6b2b8b34215a80c63a9c1348e704a0e3803ee49f2531bd62fdb5139c4`.
- Size: 4,032 bytes.

The font defines the default A as a lower-half block. Setting its `UPWD` axis
to 350 shifts A to the upper half, matching its fixed U+2580 glyph. This makes
the fixed glyph an independent reference for native variation updates.
Applications must register these bytes explicitly; the deterministic font
collection does not register this fixture automatically.
