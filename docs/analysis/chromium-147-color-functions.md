# Chromium 147.0.7727.50: oklch(), oklab() and color-mix() from parsing to 8-bit pixels

Source reading behind [the color functions record](../progress/color-functions-2026-11-09.md),
9 November 2026. The float32 emulation it mentions is
`tools/qualification/reproducers/oklch_chromium_emulation.py`. The captures in that record
later confirmed every predicted byte that was checked, including the open questions in §6 about
the raster color space (Chromium paints `oklch(1 0 0 / 10%)` over black as 25, 26, 26).

Research notes (read-only). All sources were fetched at tag `147.0.7727.50` from
`https://chromium.googlesource.com/chromium/src/+/refs/tags/147.0.7727.50/<path>?format=TEXT`.
Skia was fetched at the revision pinned by that tag's `DEPS` (`skia_revision`
`abbe599fb3c0ef2fa82bfadbb0ddcd321f22faf0`, DEPS:315) from
`https://skia.googlesource.com/skia/+/<rev>/<path>`. Line numbers refer to those files.

Labels used below:
- **[source]**: read directly in the pinned source.
- **[computed]**: a Python float32 emulation of that source. This is not a capture of a real
  Chromium build.
- **[unverified]**: an inference that was not checked against source or a capture.

---

## 0. Pipeline in one picture

```
CSS text --parse--> blink::Color{space=kOklch|kOklab, float p0,p1,p2, float alpha, none-bits}
         (doubles in parser, narrowed to float in Color::FromColorSpace; L clamped [0,1], C>=0)
  color-mix(): CSSColorMixValue (parse) -> resolved at computed-value time (StyleBuilderConverter)
         -> Color::FromColorMix -> Color in interpolation space (oklab/oklch/srgb/...)
paint:   Color::toSkColor4f()  [gamut_map = IsBakedGamutMappingEnabled() = false by default]
         -> ToSRGB -> ToXYZD50 (oklch: LchToLab -> OklabToXYZD50) -> gfx::XYZD50ToSRGB
         -> SkColor4f{r,g,b (UNCLAMPED, may be <0 or >1), alpha}
         -> cc::PaintFlags::setColor(SkColor4f) -> SkPaint (alpha pinned only)
raster (--disable-gpu => Skia CPU):
         opaque or in-range: 8-bit = (uint)(pin(v*255f + 0.5f, 0, 255))   (per channel clip)
         translucent & out-of-[0,1]: SkRasterPipeline highp, premul in float, clamp_01, round-nearest-even
```

There is **no CSS Color 4 gamut mapping** on the default paint path. Out-of-sRGB colors are
clipped per channel in gamma-encoded sRGB, inside Skia, when the color is quantized.

---

## 1. Parsing

### 1.1 Entry points [source]
- `third_party/blink/renderer/core/css/properties/css_parsing_utils.cc`
  - `ConsumeColorInternal` (lines 2199-2253). `color-mix(` goes to `ConsumeColorMixFunction`
    (2204-2208). Every other function, including `oklch(`/`oklab(`, goes to
    `ColorFunctionParser::ConsumeFunctionalSyntaxColor` (2242-2246).
  - `ConsumeColorMixFunction` (2009-2109), `ConsumeColorInterpolationSpace` (1852-1986).
- `third_party/blink/renderer/core/css/properties/css_color_function_parser.cc` (733 lines)
  - `ConsumeFunctionalSyntaxColor` (512-731), `ConsumeChannel` (240-306), `ConsumeAlpha` (308-346),
    `MakePerColorSpaceAdjustments` (348-410), `ResolveColorChannelForNumericLiteral` (425-443),
    `ResolveAlphaForNumericLiteral` (445-458), `ChannelIsResolvable` (466-496).
- `third_party/blink/renderer/core/css/color_function.h`: `ColorFunction::kMetadataMap`,
  which holds the percentage reference values (lines 46-87).

### 1.2 Channel grammar and ranges for oklch()/oklab() [source]
- Each channel accepts `none`, a `<number>`, or a `<percentage>` (`ConsumeChannel`, 246-291).
  The hue channel (`ColorChannelIsHue`: oklch index 2) accepts `none`, an `<angle>`, or a
  `<number>` (253-275). **A percentage hue is not accepted** for absolute colors. Relative-color
  channel keywords and calc() are accepted only in relative syntax.
- Percentage reference values (`color_function.h:64-76`):
  - oklab: `{1, 0.4, 0.4}`, so `L 100% = 1.0`, `a/b 100% = 0.4`.
  - oklch: `{1, 0.4, NaN}`, so `L 100% = 1.0`, `C 100% = 0.4`, and hue percentages do not apply.
- Literal resolution happens in double (`ResolveColorChannelForNumericLiteral`, 425-443):
  ```cpp
  case ChannelType::kNumber:  return value.IsAngle() ? value.ComputeDegrees() : value.DoubleValue();
  case ChannelType::kPercentage: return (value.ClampedDoubleValue() / 100.0) * percentage_base;
  ```
  Angle units (deg/grad/rad/turn) are converted to degrees in double.
- Hue (`MakePerColorSpaceAdjustments`, 354-366): a non-finite hue becomes 0. The hue is then
  wrapped in double: `fmod(fmod(h, 360.0) + 360.0, 360.0)`.
- Alpha (`ResolveAlphaForNumericLiteral`, 445-458): a number is clamped to [0,1]; a percentage is
  `/100` and then clamped to [0,1]. **Alpha is not rounded to 8 bits for oklch/oklab.** The 8-bit
  rounding `round(a*255)/255` applies only to `kSRGBLegacy` (rgb()/rgba()/hex) at 386-388.
- The doubles are narrowed to `float` when passed to
  `Color::FromColorSpace(color_space_, channels[0..2], alpha)` (line 668-669, where
  `std::optional<double>` converts to `std::optional<float>`).
- `Color::FromColorSpace` (`platform/graphics/color.cc:239-275`) applies these clamps:
  - Alpha is clamped to [0,1] (255).
  - For oklab/oklch, **L is clamped to [0,1]** (lab/lch use [0,100]) (260-269).
  - For lch/oklch, **chroma is clamped to >= 0** (`std::max(param1_, 0.f)`, 270-272).
    Negative chroma therefore becomes 0 and keeps the hue. oklab a/b are unbounded.
  - A `none` channel is stored as 0 with its `paramN_is_none_` bit set (246-252). A `none` alpha
    is stored as alpha 0 with `alpha_is_none_` set.
  - These clamps run at parse time, so the specified-value serialization already shows the
    clamped value. For example `oklch(-0.1 0.1 10)` serializes as `oklch(0 0.1 10)`.
- Legacy comma syntax is rejected for non-legacy spaces such as oklch
  (`ConsumeFunctionalSyntaxColor`, 568-570).

### 1.3 calc() and relative colors (brief) [source]
- For Lab/OkLab/Lch/OkLch, **calc() channels are not resolved at parse time**.
  `ChannelIsResolvable` returns false for them (478-484). The value becomes a
  `CSSUnresolvedColorValue` (725-729) and is resolved at computed-value time
  (`style_builder_converter.cc:2981-2984`, `unresolved_color_value->Resolve(length_resolver)`).
- Relative syntax (`oklch(from X l c h)`) produces `CSSRelativeColorValue` (714-717). That becomes
  `StyleColor::UnresolvedRelativeColor`, which resolves at computed-value time when the origin is
  absolute (`style_builder_converter.cc:2947-2966`).
- At computed-value time `ResolveNonFiniteValues` (`color.cc:1367-1399`) maps NaN to 0 and
  ±infinity to channel bounds. It is applied to `CSSColor` values
  (`style_builder_converter.cc:2893-2897`).

### 1.4 Storage precision [source]
`blink::Color` (`platform/graphics/color.h:478-493`) holds:
- `ColorSpace color_space_` (uint8)
- four 1-bit none flags
- `float param0_, param1_, param2_, alpha_`

All storage is **float32**. Color-space math in `gfx::` and `skcms::` is float32. The exceptions are
`skcms_Matrix3x3_invert`, which inverts in double and returns float, and a few double literals
noted below.

---

## 2. color-mix()

### 2.1 Parse [source] (`css_parsing_utils.cc`)
- `in <space>` is **optional** in 147. `ConsumeColorInterpolationSpace` returns the defaults
  `{kOklab, kShorter, user_specified=false}` when there is no `in` keyword (1852-1876). A comma
  after the space is required only when `in` was given (2034-2038). A bare `in` with no valid
  space is rejected (1983-1985).
- Recognized spaces (1879-1937): `srgb`, `srgb-linear`, `display-p3`, `a98-rgb`,
  `prophoto-rgb`, `rec2020`, `lab`, `oklab`, `lch`, `oklch`, `hsl`, `hwb`, `xyz`, `xyz-d50`,
  `xyz-d65`. `display-p3-linear` and `rec2100-linear` are behind runtime flags. **`in srgb` maps
  to `Color::ColorSpace::kSRGB` (the color(srgb) space), not to legacy.**
- A hue method (`shorter|longer|increasing|decreasing hue`) is accepted only for
  hsl/hwb/lch/oklch (1944-1974). The default is shorter.
- Each color may come before or after its percentage. Literal percentages below 0% or above 100%
  are rejected. If both percentages are literal 0, parsing fails. calc() percentages pass at parse
  time (2057-2096).
- The result is a `cssvalue::CSSColorMixValue` (2103-2104). **It is not folded at parse time,
  even when both colors are absolute.** Its specified-value serialization is
  `CSSColorMixValue::CustomCSSText` (`css_color_mix_value.cc:121-155`), which omits `in oklab` when
  it is the default (128-137).

### 2.2 Resolution time [source]
- In `style_builder_converter.cc` `ResolveColorValueImpl` (2920-2945), both operands are resolved
  first, then `NormalizePercentages` runs. If both operands are absolute colors, the mix is
  **resolved at computed-value time** into a plain `Color`:
  `StyleColor(unresolved_color_mix->Resolve(Color()))`. Otherwise, for example with
  `currentcolor`, a `StyleColor::UnresolvedColorMix` is stored and resolved at used-value time
  (`style_color.cc:151-158`).
- `ResolveColorValue` (3005-3031) converts a legacy-space result of color-mix or a relative color
  to `color(srgb …)`. Together with the identical step in `Color::FromColorMix`, a mix never
  serializes as `rgb()`.

### 2.3 Percentage normalization [source] (`css_color_mix_value.cc:14-60`, all double)
```cpp
p1 = percentage1 ? clamp(p1%,0,100)/100 : 0.5;  p2 = likewise
if (p1 given && !p2) p2 = 1 - p1;  else if (p2 given && !p1) p1 = 1 - p2;
if (p1 == 0 && p2 == 0) return false;
alpha_multiplier = 1.0;
scale = p1 + p2;  if (scale != 0) { p1 /= scale; p2 /= scale; if (scale <= 1.0) alpha_multiplier = scale; }
mix_amount = p2;  if (p1 == 0.0) mix_amount = 1.0;
```
- The progress is `mix_amount = p2`. When the percentages sum to less than 100%, the alpha
  multiplier is the sum. When the sum is above 100%, the percentages are scaled down and the
  multiplier stays 1.
- At computed-value time the return value is ignored and both outputs keep their initial value
  0.0 (`style_builder_converter.cc:2925-2930`). A calc() that yields 0/0 therefore produces
  `FromColorMix(..., 0, 0)`, which is color1 with alpha 0.
- `UnresolvedColorMix` stores `percentage_` and `alpha_multiplier_` as double (`style_color.h:165-166`).
  They are narrowed to `float` in the `Color::FromColorMix(..., float percentage, float alpha_multiplier)`
  signature (`color.h:238-243`).

### 2.4 Interpolation (`Color::FromColorMix` → `Color::InterpolateColors`, `color.cc:294-313`, 525-579) [source]
1. `ConvertToColorSpaceForInterpolation(space)` is applied to each color (823-842).
   - The analogous missing components are recorded first (`GetAnalogousMissingComponents`,
     358-465). For example, oklch→oklab carries L only, and rgb-like spaces carry all three
     between each other.
   - Then `ConvertToColorSpace(space)` runs. It first calls `ResolveMissingComponents`, so `none`
     becomes 0 for the conversion (683-696, 704-706).
   - The carried-forward components are then re-marked as `none`, with value 0 (467-481).
   - If the color is already in the interpolation space, the function returns early and the
     `none` bits are kept as they are.
2. `SubstituteMissingParameters` (484-522): for each of p0, p1, p2 and alpha, if one side is
   `none` and the other is not, the `none` side takes the other side's value.
3. `PremultiplyColor` (864-879): if alpha is `none`, nothing is premultiplied. Otherwise:
   - p0 *= a, except for hsl/hwb, where p0 is the hue.
   - p1 *= a.
   - p2 *= a, except for lch/oklch, where p2 is the hue.
   - alpha_ is set to 1, and the original alpha is returned.
4. Per-component blend (547-571). A component that is `none` on both sides stays `none`.
   - Hue components use `HueInterpolation`: hsl/hwb p0 and lch/oklch p2.
   - Every other component uses `blink::Blend(float from, float to, double progress)`, which is
     `static_cast<float>(from + (to - from) * progress)`. The `(to - from)` subtraction is done in
     float, and the multiply and add in double (`platform/geometry/blend.h:34-36`).
   - Alpha is `Blend(alpha1, alpha2, p)` on the original alphas.
5. `FromColorSpace(space, …)` re-applies the L/C clamps. Then `UnpremultiplyColor` (881-894)
   divides the same components by alpha, except when alpha is `none` or `alpha == 0`.
6. `result.alpha_ *= alpha_multiplier` (304). Legacy results (only possible with `in hsl` or
   `in hwb`) are converted to `kSRGB` (309-311).

### 2.5 Hue interpolation [source] (`color.cc:316-356`, float)
- **shorter**: with `diff = h2 - h1`, if `diff > 180` then `h1 += 360`; else if `diff < -180`
  then `h2 += 360`.
- **longer**: if `0 < diff < 180` then `h1 += 360`; else if `-180 < diff <= 0` then `h2 += 360`.
- **increasing**: if `h2 < h1` then `h2 += 360`.
- **decreasing**: if `h1 < h2` then `h1 += 360`.
- The result is `fmod(fmod(Blend(h1,h2,p), 360.f) + 360.f, 360.f)` (`AngleToUnitCircleDegrees`, 152-154).

### 2.6 Powerless hue [source]
- `kAchromaticChromaThreshold = 0.02` (`color.cc:62-66`), described as "slightly higher than
  white's chroma value of 0.0188".
- When a color is **converted into** lch or oklch (`ConvertToColorSpace`, 752-785) and
  `C <= 0.02`, the hue is marked `none`.
  - So in `color-mix(in oklch, white, X)`, white's hue becomes missing and takes X's hue.
  - **A color that is already oklch is not converted**, so `oklch(1 0 0)` keeps hue `0`, which is
    not `none`. The hue is then interpolated from 0. To get a missing hue, use `oklch(1 0 none)`.
- HSL treats a color as achromatic only when `s == 0` (799-802). HWB uses `w + b >= 1` (809-812).
- Quirk: in the oklab→oklch branch (771-773), `LabToLch` uses `atan2f`, which gives a hue in
  (-180, 180]. **This branch does not normalize to [0,360)**. The other branches do (763, 778).
  `HueInterpolation`'s `DCHECK(value >= 0)` is compiled out in release builds, so a negative hue
  can enter the shorter/longer logic. This affects `color-mix(in oklch, oklab(...)…, …)` and
  relative colors built from oklab. **[source; behavioral consequence unverified]**

### 2.7 `transparent` [source]
- The `transparent` keyword resolves through `StyleColor::ColorFromKeyword` to the named color
  table, giving `Color::FromRGBA32(0x00000000)`, which is `kSRGBLegacy` (0,0,0) with alpha 0
  (`style_color.cc:437-448`).
- Converted to oklab it is exactly (0,0,0), so all of its components are premultiplied to 0.
- Nothing is carried forward, because legacy rgb has no analogous components in oklab.
- Consequence: `color-mix(in oklab, C p%, transparent)` gives exactly C's oklab with alpha
  `p * alpha_C` (with float rounding). The L, a and b values come back unchanged after
  unpremultiply.

---

## 3. Conversion for painting

### 3.1 Blink → SkColor4f [source]
- Painting calls `Color::toSkColor4f()`, for example
  `GraphicsContext::FillRect → flags.setColor(color.toSkColor4f())`
  (`platform/graphics/graphics_context.cc:729-738`). Other fill paths do the same (406, 415, 749…).
- `Color::toSkColor4f()` (`color.cc:844-847`):
  `auto [r,g,b] = ToSRGB(IsBakedGamutMappingEnabled()); return SkColor4f{r,g,b,alpha_};`
- `IsBakedGamutMappingEnabled()` reads `blink::features::kBakedGamutMapping`, which is
  **`FEATURE_DISABLED_BY_DEFAULT`** (`third_party/blink/common/features.cc:192`). The experimental
  `gfx::OklabGamutMap` (a Rec2020 polyhedral chroma reduction, `color_conversions.cc:117-225`) is
  therefore **not used** by default. Gradient stops call `ToSRGB(false)` explicitly (849-855).
- `ToSRGB` (580-621) routes oklab and oklch through `ToXYZD50` (623-670):
  - oklch: `gfx::LchToLab` then `gfx::OklabToXYZD50(l,a,b,false)`.
  - oklab: `OklabToXYZD50` directly.
  - The XYZ result then goes through `gfx::XYZD50ToSRGB`.
- **The SkColor4f r, g, b are not clamped.** Out-of-gamut values stay below 0 or above 1, and the
  transfer function is sign-extended.

### 3.2 Exact math (all float32 unless noted) [source] (`ui/gfx/color_conversions.cc`)
**LchToLab** (265-268):
`a = c * std::cos(base::DegToRad(h)); b = c * std::sin(base::DegToRad(h));`
- `DegToRad<float>` is `deg * std::numbers::pi_v<float> / 180` (`base/numerics/angle_conversions.h:15-17`),
  evaluated as `(h*pi_f)/180` in float.
- `cos` and `sin` are the float overloads (cosf/sinf).

**OklabToXYZD50** (227-249):
1. `lms = Matrix3x3_apply(kOklab_to_LMS, {l,a,b})`
   ```
   kOklab_to_LMS (float literals, 51-57) =
   { 0.99999999845051981432f,  0.39633779217376785678f,  0.21580375806075880339f},
   { 1.0000000088817607767f,  -0.1055613423236563494f,  -0.063854174771705903402f},
   { 1.0000000546724109177f,  -0.089484182094965759684f,-1.2914855378640917399f}
   ```
2. `lms_cubed = {l*l*l, m*m*m, s*s*s}` (evaluated `(x*x)*x`).
3. `xyz_d65 = Matrix3x3_apply_inverse(kXYZD65_to_LMS, lms_cubed)`. The forward matrix
   (float literals, 46-49) is:
   ```
   { 0.8190224432164319f,   0.3619062562801221f,  -0.12887378261216414f},
   { 0.0329836671980271f,   0.9292868468965546f,   0.03614466816999844f},
   { 0.048177199566046255f, 0.26423952494422764f,  0.6335478258136937f}
   ```
   The inverse is computed at run time by `skcms_Matrix3x3_invert`, which works in double
   (cofactor/determinant) and casts each entry to float (`skcms.cc:1917-1972`). Note: Chromium
   does **not** use Ottosson's published LMS→XYZ matrix. It inverts this XYZ→LMS matrix.
4. `xyz_d50 = Matrix3x3_apply(getXYDZ65toXYZD50matrix(), xyz_d65)`. The adaptation matrix is
   `skcms_AdaptToXYZD50(0.3127f, 0.3290f)`, a Bradford adaptation computed in float
   (`skcms.cc:1831-1870`):
   - Source white: `wXYZ = {wx/wy, 1, (1-wx-wy)/wy}`.
   - D50 white: `{0.96422f, 1.0f, 0.82521f}`.
   - Bradford matrix: `{{0.8951,0.2664,-0.1614},{-0.7502,1.7135,0.0367},{0.0389,-0.0685,1.0296}}`.
   - Its inverse is hard-coded as `{{0.9869929,-0.1470543,0.1599627},{0.4323053,0.5183603,0.0492912},{-0.0085287,0.0400428,0.9684867}}`.
   - The matrix is assembled as `concat(lms_to_xyz, concat(diag(dst/src), xyz_to_lms))`.
   - Computed result **[computed]**:
     `[[1.04785728, 0.02290732, -0.05016222], [0.02957045, 0.99047565, -0.01706151], [-0.00924047, 0.01505292, 0.75197095]]`.

**XYZD50ToSRGB** (282-287):
`c = Matrix3x3_apply_inverse(SkNamedGamut::kSRGB, c); c = TransferFunction_apply_inverse(SkNamedTransferFn::kSRGB, c);`
- `SkNamedGamut::kSRGB` (`skia include/core/SkColorSpace.h:234-242`) is the ICC 16.16 fixed-point
  D50 sRGB matrix. Each entry is `SkFixedToFloat(x) = x * 1.52587890625e-5f`, which is exact
  (SkFixed.h:41).
  ```
  {0x6FA2, 0x6299, 0x24A0}/65536  = {0.436065674, 0.385147095, 0.143066406}
  {0x38F5, 0xB785, 0x0F84}/65536  = {0.222488403, 0.716873169, 0.060607910}
  {0x0390, 0x18DA, 0xB6CF}/65536  = {0.013916016, 0.097076416, 0.714096069}
  ```
  It is inverted at run time with `skcms_Matrix3x3_invert` (double, then float).
- `SkNamedTransferFn::kSRGB` (`SkColorSpace.h:133-134`), with fields {g,a,b,c,d,e,f}:
  `{2.4f, (float)(1/1.055), (float)(0.055/1.055), (float)(1/12.92), 0.04045f, 0, 0}`.
- `skcms::TransferFunction_apply_inverse` (`skia/ext/skcms_ext.cc:117-132`) uses **Chromium's own
  copies** of eval/invert, which call `std::pow` on floats. It does not use skcms's
  approximations (comment at 17-19 and 33-34, crbug.com/450045076).
  - `TransferFunction_invert` (35-71) gives **[computed]**:
    `inv = {g=1/2.4f=0.41666666, a=k=powf(a,-g)=1.13711905, b=-0.0, c=1/c=12.92, d=c*d=0.0031308050, e=1-powf(a*s+b, g) = -0.05500007, f=-0.0}`.
    Here `s = eval(src, 1.0)`, and `e` is recomputed so that `eval(inv, s) == 1`.
  - `TransferFunction_eval` (20-31):
    `sign*(|x|<=d ? c*|x|+f : powf(max(a*|x|+b,0), g)+e)`. This is the sRGB encode, odd-extended
    to negative values and to values above 1.

**Matrix3x3_apply** (`skcms_ext.cc:76-85`): `m0*v0 + m1*v1 + m2*v2`, evaluated left to right in float.
- `Matrix3x3_apply_inverse` returns its input unchanged when the matrix equals `SkNamedGamut::kXYZ`.
- `TransferFunction_apply_inverse` returns its input unchanged when the transfer function equals
  `kLinear` (87-95, 117-123).

**Build flags**: Chromium compiles with `-ffp-contract=off` (`/clang:-ffp-contract=off` on Windows)
(`build/config/compiler/BUILD.gn:653-658`). The C++ color math has no FMA contraction. Each
operation is a separately rounded IEEE float32 operation.

### 3.3 SkColor4f → 8-bit pixels (Skia CPU raster; the project's captures use `--disable-gpu`) [source]
- `SkPaint::setColor(const SkColor4f&, SkColorSpace* = nullptr)` (`SkPaint.cpp:123-128`) pins
  **only alpha** (`color.pinAlpha()`), then converts from sRGB to sRGB, which is the identity.
- Blitter choice: `SkBlitter::UseLegacyBlitter` (`SkBlitter.cpp:617-652`). With no shader and a
  non-null device color space, the legacy blitter is used only if `paint.getColor4f().fitsInBytes()`
  (all of r, g, b in [0,1]) **and** `cs->isSRGB()`. The device must also be N32, SrcOver, and not
  dithered.
  - **Legacy path** (in-gamut): `SkARGB32_Blitter` takes `paint.getColor()`, which is
    `SkColor4f::toSkColor()` → `Sk4f_toL32` (`SkSwizzlePriv.h:53-60`):
    `cast<uint8_t>(pin(px*255.f + 0.5f, 0, 255))`. That is round half up, then truncation.
    Translucent colors are premultiplied in 8 bits with `SkMulDiv255Round`, then blended with
    `blit_row_color32`: `dst*(256-a)>>8 + pmcolor` (`SkBlitRow_opts.h:243-275`).
  - **Raster-pipeline path** (out of range, or non-sRGB device): `paint_color_to_dst` →
    `appendConstantColor(color.premul())` (`SkRasterPipelineBlitter.cpp:148-176`). Then
    `appendClampIfNormalized` → `clamp_01` (410-422, `SkRasterPipeline.cpp:565-569`) clamps each
    premultiplied channel to [0,1].
    - If every channel is in [0,a], the color becomes `uniform_color` with
      `(uint16_t)(c*255+0.5)`, which is lowp (`SkRasterPipeline.cpp:275-303`).
    - Otherwise it stays highp `unbounded_uniform_color`. Stores then use
      `to_unorm` = `round(min(max(0, v*255), 255))`, where x86 `round` is `_mm*_cvtps_epi32`
      (round to nearest even) (`SkRasterPipeline_opts.h:2038-2044`, 614, 812).
    - Opaque constant colors become a memset of that stored value (440-470).
- **Net effect for opaque colors**: `byte = (int)(clamp(v,0,1)*255f + 0.5f)` per channel. Clipping
  is per channel in gamma-encoded sRGB. Pipeline differences can only matter at exact .5 ties.
- **Translucent colors with a channel slightly above 1** go down a different path. An example is
  `oklch(1 0 0 / 10%)`, whose SkColor4f is (0.99987, 1.00005, 1.00007) **[computed]**. Such a color
  does not `fitsInBytes`, so it takes the float raster pipeline rather than the 8-bit legacy
  blitter. The results differ from `rgba(255,255,255,.1)` (see §5).
- GPU raster is not used in these captures. On a GPU, the shader blends uniform floats and the
  hardware quantizes the result. **[unverified]**

### 3.4 `--force-color-profile=srgb` [partly unverified]
- On the Blink side nothing changes. `toSkColor4f` always produces sRGB-relative values, and
  `SkPaint::setColor` treats them as sRGB.
- The flag pins the display color space, and therefore the raster color space, to sRGB. That keeps
  the paint→device transform the identity and keeps `cs->isSRGB()` true, so the legacy blitter
  stays eligible.
- Without the flag, a wide-gamut display profile could change the raster color space, or the
  cc content-color-usage logic could, and the bytes would then change. I did not trace the cc and
  viz code for this in 147. **[unverified]** The project's `tests/pixel_comparison/capture_screenshots.js:26`
  passes the flag. The qualification scripts pass `--headless --disable-gpu` but not this flag.
  Headless normally reports an sRGB screen. **[unverified]**

---

## 4. Computed-value serialization (getComputedStyle) [source]

- `getComputedStyle` → `ComputedStyleUtils::ValueForColor` (`computed_style_utils.cc:190-199`)
  → `CSSColor::Create(style_color.Resolve(current_color, …))`. Any stored unresolved mix is
  resolved here.
- The value is then printed by `Color::SerializeAsCSSColor()` (`color.cc:1110-1116`):
  - Legacy spaces print as `rgb()`/`rgba()`.
  - Everything else goes through `SerializeInternal` (1082-1108):
    - lab/oklab/lch/oklch print as `oklch(L C H)`, `oklab(L a b)`, and so on.
    - Predefined spaces print as `color(srgb r g b)`, `color(srgb-linear …)`, `color(xyz-d65 …)`, and so on.
    - A `none` component prints as `none`.
    - ` / A` is appended when `alpha != 1` or alpha is `none`.
- Number format: `ColorParamToString(param, precision=6)` (997-1014), and alpha uses
  `AppendNumber(float)`, also 6 digits.
  - Both go through `DoubleToStringConverter::ToStringWithFixedPrecision(d, 6)`
    (`wtf/dtoa.cc:125-147`): the EcmaScript `toPrecision(6)` (exponent form only when the
    exponent is below -6 or at least 6), then trailing zeros are stripped. Mixed results can
    therefore show `e-7`-style numbers.
  - The value printed is the float widened to double. For example, `0.7f` prints as `0.7`.
  - Non-finite values print as `calc(infinity)` and similar.
- Results:
  - `oklch(0.7 0.1 200)` serializes as **`oklch(0.7 0.1 200)`**. Specified and computed values
    match, because no conversion happens.
  - `oklch(1 0 0 / 10%)` serializes as `oklch(1 0 0 / 0.1)`.
  - `color-mix(in oklab, red 30%, blue)` serializes as **`oklab(…)`**, the result in the
    interpolation space. **[computed]**: `oklab(0.504777 0.0447402 -0.180308)` with the float
    pipeline (red = oklab(0.627966 0.22488 0.125859), blue = oklab(0.451981 -0.0324624 -0.311523),
    progress 0.7).
  - `color-mix(in srgb, …)` serializes as `color(srgb r g b)`.
  - `in srgb-linear` serializes as `color(srgb-linear …)`.
  - `in oklch` serializes as `oklch(…)`.
  - `in hsl`/`in hwb` serialize as `color(srgb …)` (`FromColorMix`, 306-311).
  - `in lab` serializes as `lab(…)`.
  - Mixes never serialize as `rgb()`.
  - Specified style (`el.style.color`) keeps the `color-mix(...)` text from `CustomCSSText`. The
    default `in oklab` is omitted there.

---

## 5. Worked values (Tailwind v4 and shadcn tokens)

These were produced by `tools/qualification/reproducers/oklch_chromium_emulation.py`, which emulates §3.2 and §3.3 operation by
operation in numpy float32. It treats powf, cosf and sinf as correctly rounded. The platform libm (UCRT) may differ by 1 ULP,
which is irrelevant here because every case below is at least 0.08 away from a rounding boundary.
**All values are [computed], not verified against a Chromium capture.**

### Opaque colors

| color | SkColor4f r,g,b (float) | 8-bit |
|---|---|---|
| `oklch(0.205 0 0)` | 0.09050950, 0.09053466, 0.09053701 | **rgb(23,23,23)** (#171717) |
| `oklch(0.985 0 0)` | 0.98012900, 0.98030794, 0.98032486 | **rgb(250,250,250)** (#fafafa) |
| `oklch(0.577 0.245 27.325)` | 0.90641546, **-0.09585108**, 0.04235788 | **rgb(231,0,11)** (#e7000b) |

For `oklch(0.577 0.245 27.325)`, green is out of gamut and is clipped to 0 in Skia.

Chromium's oklab→sRGB result is not exactly neutral for a=b=0. For example R < G < B by about
3e-5 for L=0.205. The cause is the float matrices and D50/ICC round-trip. It does not affect the
bytes in these cases. These match the Tailwind palette hexes (neutral-900 #171717, neutral-50
#fafafa, red-600 #e7000b), which is a consistency check only.

### Translucent colors

**`oklch(1 0 0 / 10%)`**
- SkColor4f is (0.99987054, 1.00005305, 1.00007010) with alpha 0.1f. The G and B channels are
  above 1.
- The unpremultiplied SkColor would be (255,255,255,26).
- Because the color does not `fitsInBytes`, it takes the **raster-pipeline highp path** (§3.3).
  The premultiplied float is (0.0999871, 0.1000053, 0.1000070), and stores round to nearest:

| backdrop | raster-pipeline result | legacy rgba(255,255,255,.1) |
|---|---|---|
| black | **rgb(25,26,26)** (R=25.4967→25, G=25.5014→26) | rgb(26,26,26) |
| rgb(23,23,23) | rgb(46,46,46) | rgb(46,46,46) |
| white | rgb(255,255,255) | rgb(255,255,255) |

The black-backdrop difference is a precise prediction worth checking with a capture.

**`color-mix(in oklab, oklch(0.205 0 0) 50%, transparent)`**
- Percentages normalize to p1=0.5 and p2=0.5, giving mix 0.5 and alpha multiplier 1.
- oklch converts to oklab as (0.205f, 0, 0). Transparent is (0,0,0) with alpha 0.
- Premultiplied: L = 0.205·1 and 0·0. Blend: L = 0.1025, alpha = 0.5. Unpremultiplied:
  L = 0.205f exactly.
- Result: **`oklab(0.205 0 0 / 0.5)`** (computed value). SkColor4f is the same RGB as
  `oklch(0.205 0 0)` with alpha 0.5.
- The 8-bit unpremultiplied value is (23,23,23,128), with alpha 0.5·255+0.5 = 128.
- Over white (legacy blitter): premultiplied 12 → `255*(256-128)>>8 + 12` = **rgb(139,139,139)**.
  Over black: **rgb(12,12,12)**. This is the same as `rgba(23,23,23,0.5)` would produce.

---

## 6. Remaining uncertainty

1. **No capture verification.** All 8-bit numbers come from emulation. The next check is to
   capture swatches with the project's flags and compare. The highest-value cases are
   `oklch(1 0 0 / 10%)` over black, `oklch(0.577 0.245 27.325)`, and a
   `color-mix(... transparent)` swatch.
2. **The cc raster surface color space** (null vs sRGB SkColorSpace) under `--disable-gpu` was not
   traced in cc. If it is null, the legacy blitter is used even for out-of-range colors. Opaque
   bytes stay identical, but `oklch(1 0 0/10%)` over black would become 26,26,26 instead of
   25,26,26. **[unverified]**
3. **Which SkRP CPU variant runs.** HSW uses FMA in `mad`, and SSE2 does not. Round-to-even vs
   +0.5 differs only at exact ties, and I found no ties in the cases above.
4. **Wide-gamut raster color space** without `--force-color-profile=srgb` was not traced.
5. **powf/cosf/sinf** from the Windows UCRT may differ by 1 ULP from the correctly rounded values
   I assumed.
6. **Negative hue from the oklab→oklch conversion** (§2.6) is a source fact. Its effect on mixes
   was not exercised.

---

## 7. Follow-up

The script was extended with the reverse path, a color-mix() emulation, skcms's approximate
transfer functions and the Skia CPU compositing helpers. It is `tools/qualification/reproducers/oklch_chromium_emulation.py`; run it with `python -I`.
`section5()` reproduces §5 and `followup()` reproduces §7.3.

### 7.1 Type of the percentage reference values [source]

`third_party/blink/renderer/core/css/color_function.h:21-23`:
```cpp
// The value (number) that equals 100% for the corresponding positional color channel.
std::array<double, 3> channel_percentage;
```

The entries are the double literals `{1, 0.4, 0.4}` for oklab (66) and `{1, 0.4, kPercentNotApplicable}`
for oklch (76). **The base 0.4 is the double 0.4, not 0.4f widened.**

The parser computes `(value.ClampedDoubleValue() / 100.0) * percentage_base` entirely in double
(`css_color_function_parser.cc:437-438`). It narrows to float only once, when the
`std::optional<double>` channels are passed to `Color::FromColorSpace(..., std::optional<float> ...)`
(668-669). For example, `C 50%` becomes `(50/100.0)*0.4 = 0.2` in double, then `float(0.2)`.

### 7.2 Reverse path: sRGB, srgb-linear and legacy colors to oklab/oklch, and back to SkColor4f [source]

#### How the legacy 8-bit channels enter

**Hex and named colors.** These use `Color::FromRGBA32`, which is the `Color(RGBA32)` constructor
(`color.h:412-420`):
```cpp
param0_(((color >> 16) & 0xFF)), param1_(((color >> 8) & 0xFF)), param2_(((color >> 0) & 0xFF)),
alpha_(((color >> 24) & 0xFF) / 255.f)
```
- The channels are stored as floats in 0..255 inside `kSRGBLegacy`.
- Alpha is computed as the float `a/255.f`.
- `transparent` and `white` arrive this way (`style_color.cc:442-443`).

**rgb() and rgba().** These are parsed as doubles and clamped to [0,255]. Fractional values are
kept. Alpha is rounded with `round(a*255)/255` (`css_color_function_parser.cc:368-389`). The values
are then stored as floats via `FromColorSpace(kSRGBLegacy, …)`.

**Normalization to [0,1].** `gfx::SRGBLegacyToSRGB` (`color_conversions.cc:278-280`) does:
```cpp
return std::make_tuple(r / 255.0, g / 255.0, b / 255.0);
```
This divides in double and the return type narrows to float, so the result is the correctly rounded
`float(r/255)`. **It is not `r/255.f`.** For the 0..255 integers the two give the same value, but the
double version is what the source does.

#### Legacy or `color(srgb …)` to XYZ-D50

- `kSRGBLegacy`: `Color::ToXYZD50` (`color.cc:625-628`) runs `SRGBLegacyToSRGB` and then
  `gfx::SRGBToXYZD50`.
- `kSRGB`, `kSRGBLinear` and the other predefined spaces (629-646) do:
  ```cpp
  GetSkColorSpaceParams(color_space_, m, t);   // kSRGB: m=SkNamedGamut::kSRGB, t=SkNamedTransferFn::kSRGB (168-171)
                                               // kSRGBLinear: m=kSRGB, t=SkNamedTransferFn::kLinear (172-175)
  c = skcms::TransferFunction_apply(t, c);
  c = skcms::Matrix3x3_apply(m, c);
  ```
- `gfx::SRGBToXYZD50` (`color_conversions.cc:289-294`) is identical:
  `TransferFunction_apply(SkNamedTransferFn::kSRGB)` then `Matrix3x3_apply(SkNamedGamut::kSRGB)`.
- `TransferFunction_apply` is Chromium's accurate float copy (`skcms_ext.cc:20-31, 108-115`):
  ```cpp
  |x| <= d ? sign*(c*|x| + f) : sign*(std::pow(max(a*|x|+b,0), g) + e)
  ```
  With `kSRGB` (g=2.4, a=1/1.055, b=0.055/1.055, c=1/12.92, d=0.04045) this is the sRGB decode
  with float powf.
- The matrix is the D50 ICC fixed-point sRGB matrix listed in §3.2.
- `kLinear` = `{1,1,0,0,0,0,0}` gives `sign*(|x| + 0)`, which is the identity.

#### XYZ-D50 to Oklab

`gfx::XYZD50ToOklab` (`color_conversions.cc:251-263`):
```cpp
const auto xyz_d65  = skcms::Matrix3x3_apply_inverse(getXYDZ65toXYZD50matrix(), xyz);
const auto lms_cubed = skcms::Matrix3x3_apply(kXYZD65_to_LMS, xyz_d65);
const skcms::Vector3 lms{{ powExt(lms_cubed.vals[0], 1.0f / 3.0f), ... }};
const auto oklab = skcms::Matrix3x3_apply_inverse(kOklab_to_LMS, lms);
```

1. **D50 to D65.** The adaptation matrix is not hard-coded. `skcms_AdaptToXYZD50(0.3127f, 0.3290f)`
   builds the float Bradford D65→D50 matrix (§3.2). `skcms::Matrix3x3_apply_inverse`
   (`skcms_ext.cc:87-95`) then inverts it through `skcms_Matrix3x3_invert`, which computes in double
   and casts to float (`skcms.cc:1917-1972`). This happens on every call.

   Computed inverse **[computed]**:
   `[[0.955536664, -0.0230600424, 0.0632183999], [-0.0283153262, 1.00995123, 0.0210259948], [0.0123087717, -0.0205005296, 1.33019459]]`.
2. **XYZ to LMS.** This is the `kXYZD65_to_LMS` float matrix (46-49).
3. **Cube root.** `powExt(x, 1.0f/3.0f)` (30-36) is `x < 0 ? -powf(-x, p) : powf(x, p)` with
   `p = 0.333333343f`. It is **powf with a float exponent, not `std::cbrt`**. The rounded exponent
   makes the result differ from a true cube root by a relative amount of about `|ln x|·1e-8`.
4. **LMS to Oklab.** No published LMS→Oklab constants appear anywhere. The code inverts
   `kOklab_to_LMS` at run time.

   Computed inverse **[computed]**:
   `[[0.21045427, 0.793617785, -0.00407204311], [1.9779985, -2.42859221, 0.45059371], [0.0259040315, 0.782771707, -0.808675766]]`.
   These are Ottosson's M2 values to about 7 digits.

#### Oklab to OkLCh

`gfx::LabToLch` (`color_conversions.cc:269-272`):
```cpp
return std::make_tuple(l, std::sqrt(a * a + b * b), base::RadToDeg(atan2f(b, a)));
```
- The square root is the float `sqrtf(a*a + b*b)`.
- `base::RadToDeg<float>` is `rad * 180 / std::numbers::pi_v<float>`, evaluated as `(rad*180)/pi_f`
  in float (`base/numerics/angle_conversions.h:21-23`).
- The caller `ConvertToColorSpace(kOklch)` (`color.cc:770-785`) has two branches:
  - From any space other than oklab, it wraps the hue with `AngleToUnitCircleDegrees` (778).
  - From oklab it does **not** wrap the hue (771-773).
  - Both branches mark the hue `none` when `C <= 0.02f`.

#### Conversions to srgb and srgb-linear (for `color-mix(in srgb|srgb-linear, …)`)

These are done by `ConvertToColorSpace`:
- **To kSRGB** (786-788): `ToSRGB()`.
  - Legacy colors go through `SRGBLegacyToSRGB`.
  - oklab, oklch and the others go through `ToXYZD50`, then `XYZD50ToSRGB`: the inverse kSRGB matrix
    followed by Chromium's accurate inverse transfer function (§3.2).
- **To kSRGBLinear** (709-729):
  ```cpp
  GetSkColorSpaceParams(kSRGBLinear, m, t);       // m = kSRGB gamut, t = kLinear
  auto [x, y, z] = ExportAsXYZD50Floats();
  c = skcms::Matrix3x3_apply_inverse(m, c);
  c = skcms::TransferFunction_apply_inverse(t, c); // early-outs: Equal(trfn, kLinear) → identity
  ```
  The value goes through XYZ-D50 and the inverse D50 sRGB matrix. **Legacy colors take this route
  too**, so `blue` comes out with a tiny negative green, about -3.4e-9.

#### Mix result back to SkColor4f

All of these go through `Color::toSkColor4f` → `ToSRGB(false)` (`color.cc:580-621`):
- **kOklab / kOklch**: `ToXYZD50` then `gfx::XYZD50ToSRGB`, as in §3.2.
- **kSRGB**: `return std::make_tuple(param0_, param1_, param2_);` (582-583). The floats are passed
  through unchanged.
- **kSRGBLinear** (586-595) uses **skcms's own approximate math**, not Chromium's accurate copy:
  ```cpp
  // Several SVG rendering tests expect the inaccurate results from this formulation ... crbug.com/450045076
  skcms_TransferFunction tf_inv;
  skcms_TransferFunction_invert(&SkNamedTransferFn::kSRGB, &tf_inv);
  return std::make_tuple(skcms_TransferFunction_eval(&tf_inv, param0_), ...);
  ```
  - `skcms_TransferFunction_eval` (`skcms.cc:250-297`, sRGBish branch 282-284) computes
    `sign*(x < d ? c*x+f : powf_(a*x+b, g)+e)`. Note the test is `<`, not `<=`.
  - `powf_` (93-100) is `exp2f_(log2f_(x)*y)`, built from polynomial and rational approximations:
    - `log2f_` (44-59): `e - 124.225514990f - 1.498030302f*m - 1.725879990f/(0.3520887068f + m)`.
    - `exp2f_` (65-90): `(1<<23)*(x + 121.274057500f - 1.490129070f*fract + 27.728023300f/(4.84252568f - fract))`,
      then the bits are reinterpreted.
  - `skcms_TransferFunction_invert` (1988-2101) uses `powf_` as well.
  - The resulting inverse is **[computed]**:
    `{g=0.41666666, a=1.13728333, b=-0, c=12.92, d=0.0031308050, e=-0.05496979, f=-0}`.
    Chromium's accurate inverse has `a=1.13711905, e=-0.05500007`. The coefficients differ, and
    every `powf_` call adds its own error of up to about 1e-4 relative.
  - The emulation reproduces this bit by bit, assuming the same float ops.
  - Gradients and `ConvertToColorSpace(kSRGB)` from srgb-linear use the accurate path. Only
    painting an srgb-linear color directly uses this approximate one.

### 7.3 Worked color-mix() cases [computed]

These come from the extended script. 8-bit values assume Skia CPU raster with an sRGB device, as in
§3.3. "legacy" means the 8-bit `SkARGB32` blitter. "SkRP" means the raster pipeline.

| expression | computed value (serialization) | SkColor4f r, g, b / α | 8-bit unpremul | over white | over black |
|---|---|---|---|---|---|
| `color-mix(in oklab, white 10%, transparent)` | `oklab(0.999994 4.55678e-05 2.00868e-05 / 0.1)` | 1.00000060, 0.99999988, 1.00000012 / 0.100000024 | (255,255,255,26) | 255,255,255 (SkRP highp, because R > 1) | 26,26,26 (SkRP) |
| `color-mix(in oklab, #3b82f6 50%, transparent)` | `oklab(0.623066 -0.0332245 -0.185038 / 0.5)` | 0.23137331, 0.50980359, 0.96470594 / 0.5 | (59,130,246,128) | 157,192,250 (legacy) | 30,65,123 (legacy) |
| `color-mix(in oklch, oklch(0.7 0.1 200), white)` | `oklch(0.849997 0.0500249 200)` | 0.65891141, 0.84799582, 0.85728908 / 1 | (168,216,219,255) | opaque | opaque |
| `color-mix(in srgb, red 30%, blue)` | `color(srgb 0.3 0 0.7)` | 0.30000001, 0, 0.69999999 / 1 | **(77,0,179,255)**, see the tie note | opaque | opaque |
| `color-mix(in srgb-linear, red 30%, blue)` (extra) | `color(srgb-linear 0.3 -3.36731e-09 0.7)` | 0.58388901, -4.35e-08, 0.85433960 / 1 | (149,0,218,255) via SkRP memset, because G < 0 | opaque | opaque |

Notes on the rows:

- **white 10%.** Normalization gives `mix = 0.9`, which becomes `0.9f`, and `alpha_multiplier = 1`.
  - Alpha is `Blend(1, 0, 0.9f) = 0.10000002384`. It serializes as `0.1`.
  - White converts to `oklab(0.999994 4.56e-5 2.01e-5)`, which is not exactly (1, 0, 0). Its
    unpremultiplied components come back unchanged, so the computed value keeps the tiny a and b.
    `getComputedStyle` prints them in exponent form.
- **#3b82f6 50%.** The round trip through oklab returns nearly the original sRGB, and the bytes
  round back to 59/130/246.
- **oklch mix with white.** White converts to oklch as `(0.999994, 4.98e-05, hue)`.
  - Its chroma is `<= 0.02`, so the hue is marked `none` and then takes the value 200 from
    oklch(0.7 0.1 200) (`SubstituteMissingParameters`).
  - L and C interpolate as premultiplied values at alpha 1.
  - For reference, `oklch(0.7 0.1 200)` alone gives (64,177,183).
- **srgb red 30%, blue.** `p1 = 0.3`, `p2 = 1 - 0.3` (double), giving `mix = 0.7000000000000001`,
  which becomes `0.7f`.
  - `r = Blend(1, 0, 0.7f) = 0.30000001192f` and `b = 0.69999998808f`.
  - **Tie warning:** in float, `r*255.f == 76.5` and `b*255.f == 178.5` exactly. The legacy opaque
    blitter's `Sk4f_toL32` (`px*255.f + 0.5f`, then truncate) gives **77 and 179**.
  - A path that rounds to nearest even (SkRP highp `_mm_cvtps_epi32`), or a fused multiply-add of
    `v*255+0.5`, would give 76 and 178. Chromium builds with `-ffp-contract=off`, and this color
    takes the legacy path because it fits in bytes on an sRGB device. **77, 0, 179 is the source
    prediction.**
  - A Rust implementation must use `(v*255f32 + 0.5f32) as u8` with separate rounding.

### 7.4 CSS number tokens to double [source]

- `CSSTokenizer::ConsumeNumber` (`core/css/parser/css_tokenizer.cc:303-358`) finds the extent of the
  number per the CSS grammar: sign, digits, `.digits`, then `e`/`E` with an optional sign and
  digits.
  - **Integers** (`kIntegerValueType`) take the fast path `GetNaturalNumberAsDouble`
    (`css_tokenizer_input_stream.cc:44-62`). When the string is 8-bit and the digits number at most
    14, it computes `result = result * 10 + digit` in double, which is exact because the value is
    below 2^53. Otherwise it falls back to `GetDouble`.
  - **Other numbers** use `GetDouble` (31-41) → `CharactersToDouble`
    (`wtf/text/string_to_number.cc:244-275`) → `ParseDouble` (`wtf/dtoa.cc:171-178`) →
    `double_conversion::StringToDoubleConverter::StringToDouble`. The converter is constructed in
    `GetDoubleConverter` (57-63) with `ALLOW_LEADING_SPACES | ALLOW_TRAILING_JUNK`.
- Google's double-conversion `StringToDouble` is **correctly rounded** (round to nearest, like
  `strtod`). It falls back to bignum arithmetic for hard cases. That correctness guarantee comes from
  the library, not from Chromium code. **[library property, not re-verified here]**
- The value is kept as a double in `CSSParserToken` and `CSSNumericLiteralValue`. Color channels
  narrow it to float only in `Color::FromColorSpace`. **Rust's `str::parse::<f64>` is also correctly
  rounded**, so it matches.
- A percentage token (`50%`) carries the number 50.0. Percent arithmetic such as `/100.0` happens
  later, in double (§1.2 and §2.3).

### 7.5 Additional uncertainty

- **Skia's compile flags.** I assumed Skia is built with Chromium's `-ffp-contract=off` because it
  is compiled through Chromium's default `compiler` config. I did not check this in
  `skia/BUILD.gn`. If Skia were compiled with contraction, the srgb red/blue tie case would
  change to 76/178.
- **`ConvertToColorSpace(kOklch)` from oklab** skips hue normalization (§2.6). The script follows
  the source, but no case above exercises it.
- **No capture verification.** As before, none of the §7.3 bytes have been checked against a
  Chromium capture.
