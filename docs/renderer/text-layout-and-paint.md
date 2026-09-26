# Text layout and paint contract

Typography declarations enter the renderer through the generated property
schema. The computed value is consumed by a single path shared by intrinsic
sizing, line breaking, fragment construction, painting, hit testing,
selection, and accessibility.

Viewport-relative lengths retain their `vw`, `vh`, `vmin`, or `vmax` identity
in authored values. They are resolved from the active logical viewport; the
WPT porter is forbidden from substituting the legacy 800 by 600 profile.
Transforms and static-geometry probes that cannot retain a viewport-relative
expression are rejected instead of being pre-evaluated.

Discretionary and dictionary hyphenation measure and paint the configured
`hyphenate-character`. Soft-hyphen source characters remain invisible and are
skipped exactly once after a taken break. Text autospace is part of shaping
advance data, so the one-eighth-em ideograph/alphanumeric gap is visible to
line breaking and caret geometry. Explicit `text-spacing-trim: trim-start`
removes the half-em opening-punctuation space in the shaped run.

`text-size-adjust` percentages alter the used font strike while retaining the
specified CSS size for font selection and diagnostics. `text-box-trim` is
applied to the first and last line fragments using the selected font's text,
cap, x-height, ideographic, or alphabetic edge. It therefore changes intrinsic
block size and exported baselines, not only paint coordinates.

First-line and first-letter runs receive the complete supported font/text
bundle, including variation, feature, palette, synthesis, wrapping,
decoration, emphasis, ruby, locale, and text-box inputs. Native list markers
and form placeholders consume their typed pseudo style for font metrics and
color; absent placeholder styles keep the user-agent gray.

All geometry described here is in logical CSS pixels. The immutable scene is
replayed to the authoritative physical surface under the device-scale
transform established by the viewport contract. Software and OpenGL
presentation consume that same physical-resolution output and never rescale a
completed frame.
