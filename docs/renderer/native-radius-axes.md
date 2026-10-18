# Native independent radius axes

Native applications can author horizontal and vertical lengths independently
for each physical corner. The eight appended property IDs are 251–258 in
TL x/y, TR x/y, BR x/y, BL x/y order. Existing property IDs and C layouts remain
unchanged. These are native component APIs; the corresponding native literal
tokens do not introduce additional browser CSS longhands or a script runtime.

Rust applications use the generated `Element::set_border_top_left_radius_x`
and seven neighboring setters with `LengthValue`. All native length units use
the ordinary Engine font and viewport resolution. Finite pixel-plus-percentage
calc inputs retain both coefficients. Simple negative lengths, auto/none and
nonfinite inputs fail before mutation. Used calc radii clamp to zero after the
actual box basis is available; the shared contour consumer applies overlap
reduction, physical border contraction and continuation policy.

`Element::radius_axis_length(property)` reflects the current computed source
length. Font and viewport lengths become pixels; percentage and calc inputs
remain percentage or calc. This read does not apply overlap reduction or return
a raster pixel radius. Scene snapshots retain their prior owned source inputs
and pixels after later mutations or document destruction.

C and C++ applications use `OuiRadiusAxisLengthV1` with its size and global ABI
header. The 24-byte prefix stores native f32 coefficients and existing length
unit numbers. Descriptor-local unit 13 means `calc(value% + calc_offset px)`.
`oui_style_value_radius_axis_length_v1` creates an immutable, property-bound
owned carrier; set it with `oui_element_set_property` and destroy the carrier
with `oui_style_compound_destroy`. The Engine owns its copied source. Reusing a
carrier for another property is rejected. No caller input pointer is retained.
`oui_element_radius_axis_length_v1` validates the initialized short header
before reading the full known prefix, writes only on success, and leaves future
tails untouched. Live handles and calls remain confined to their owner thread.

The circular `BorderRadius` shorthand and legacy paired corner transports
still work. Canonical ownership now distinguishes each x/y source, so later
axis writes or removal mask only that axis and never revive an older shorthand.
The separate legacy `BorderRadiusPercent` computed override remains an ordered
numeric/flag overlay, including its established raw-source clearing behavior.
Full surviving legacy paired declarations and discrete effects preserve their
original source presence and accepted raw numeric operands. Newer native axis
effects publish only their own validated/clamped components after that pair;
partial declarations project just the surviving component.

Native length effects fold shorthand and axis components through the existing
shared scalar effect stack. Replace, Add and Accumulate use current authored
underlying values and surviving lower effects. Legacy renderer corner pairs
retain their discrete interpolation and percentage-flag behavior. Final value
publication, cancellation, source removal and callback delivery continue
through the shared Engine; there is no separate radius animation engine.

The native Engine, public Rust, C and C++ guards cover asymmetric sources,
actual-box resizing, inherited fonts, viewport units, ordered overlays, effects,
removal, error nonmutation, retained snapshots and lifetime. New guards require
current-source compilation and execution by the workspace owner. They do not
qualify Chromium coverage, independent dynamic layer tracks, or the separately
recorded rounded-mask coverage questions.
