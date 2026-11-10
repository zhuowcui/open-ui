"""Emulate Chromium 147.0.7727.50 oklch/oklab -> SkColor4f -> 8-bit, in float32.

Follows ui/gfx/color_conversions.cc, skia/ext/skcms_ext.cc,
third_party/blink/renderer/platform/graphics/color.cc and skcms.cc exactly
(operation order, float vs double). powf/cosf/sinf are emulated as the
correctly rounded float32 of the double-precision result.
"""
import math
import numpy as np

f = np.float32


def powf(x, y):
    return f(math.pow(float(x), float(y)))


def cosf(x):
    return f(math.cos(float(x)))


def sinf(x):
    return f(math.sin(float(x)))


def mat(rows):
    return [[f(v) for v in r] for r in rows]


def apply(m, v):
    # skcms::Matrix3x3_apply: ((m0*v0) + (m1*v1)) + (m2*v2), float
    return [f(f(f(m[r][0] * v[0]) + f(m[r][1] * v[1])) + f(m[r][2] * v[2])) for r in range(3)]


def invert(src):
    # skcms_Matrix3x3_invert (double math, float result)
    a00 = float(src[0][0]); a01 = float(src[1][0]); a02 = float(src[2][0])
    a10 = float(src[0][1]); a11 = float(src[1][1]); a12 = float(src[2][1])
    a20 = float(src[0][2]); a21 = float(src[1][2]); a22 = float(src[2][2])
    b0 = a00 * a11 - a01 * a10
    b1 = a00 * a12 - a02 * a10
    b2 = a01 * a12 - a02 * a11
    b3 = a20; b4 = a21; b5 = a22
    det = b0 * b5 - b1 * b4 + b2 * b3
    invdet = 1.0 / det
    b0 *= invdet; b1 *= invdet; b2 *= invdet; b3 *= invdet; b4 *= invdet; b5 *= invdet
    d = [[None] * 3 for _ in range(3)]
    d[0][0] = f(a11 * b5 - a12 * b4)
    d[1][0] = f(a02 * b4 - a01 * b5)
    d[2][0] = f(+b2)
    d[0][1] = f(a12 * b3 - a10 * b5)
    d[1][1] = f(a00 * b5 - a02 * b3)
    d[2][1] = f(-b1)
    d[0][2] = f(a10 * b4 - a11 * b3)
    d[1][2] = f(a01 * b3 - a00 * b4)
    d[2][2] = f(+b0)
    return d


def concat(A, B):
    return [[f(f(f(A[r][0] * B[0][c]) + f(A[r][1] * B[1][c])) + f(A[r][2] * B[2][c])) for c in range(3)] for r in range(3)]


def adapt_to_xyzd50(wx, wy):
    wx = f(wx); wy = f(wy)
    wXYZ = [f(wx / wy), f(1), f(f(f(f(1) - wx) - wy)) / wy]
    wXYZ[2] = f(wXYZ[2])
    wD50 = [f(0.96422), f(1.0), f(0.82521)]
    xyz_to_lms = mat([[0.8951, 0.2664, -0.1614], [-0.7502, 1.7135, 0.0367], [0.0389, -0.0685, 1.0296]])
    lms_to_xyz = mat([[0.9869929, -0.1470543, 0.1599627], [0.4323053, 0.5183603, 0.0492912], [-0.0085287, 0.0400428, 0.9684867]])
    src = apply(xyz_to_lms, wXYZ)
    dst = apply(xyz_to_lms, wD50)
    diag = [[f(dst[0] / src[0]), f(0), f(0)], [f(0), f(dst[1] / src[1]), f(0)], [f(0), f(0), f(dst[2] / src[2])]]
    m = concat(diag, xyz_to_lms)
    m = concat(lms_to_xyz, m)
    return m


kXYZD65_to_LMS = mat([[0.8190224432164319, 0.3619062562801221, -0.12887378261216414],
                      [0.0329836671980271, 0.9292868468965546, 0.03614466816999844],
                      [0.048177199566046255, 0.26423952494422764, 0.6335478258136937]])
kOklab_to_LMS = mat([[0.99999999845051981432, 0.39633779217376785678, 0.21580375806075880339],
                     [1.0000000088817607767, -0.1055613423236563494, -0.063854174771705903402],
                     [1.0000000546724109177, -0.089484182094965759684, -1.2914855378640917399]])
FX = f(1.52587890625e-5)
kSRGB_gamut = [[f(0x6FA2 * FX), f(0x6299 * FX), f(0x24A0 * FX)],
               [f(0x38F5 * FX), f(0xB785 * FX), f(0x0F84 * FX)],
               [f(0x0390 * FX), f(0x18DA * FX), f(0xB6CF * FX)]]
# skcms_TransferFunction {g, a, b, c, d, e, f}
kSRGB_tf = dict(g=f(2.4), a=f(1 / 1.055), b=f(0.055 / 1.055), c=f(1 / 12.92), d=f(0.04045), e=f(0), f=f(0))


def tf_eval(fn, x):
    x = f(x)
    sign = f(-1) if x < 0 else f(1)
    ax = f(abs(x))
    if ax <= fn['d']:
        return f(sign * f(f(fn['c'] * ax) + fn['f']))
    base = max(f(f(fn['a'] * ax) + fn['b']), f(0))
    if fn['g'] == f(1):
        return f(sign * f(base + fn['e']))
    return f(sign * f(powf(base, fn['g']) + fn['e']))


def tf_invert(src):
    inv = dict(g=f(0), a=f(0), b=f(0), c=f(0), d=f(0), e=f(0), f=f(0))
    d_l = f(f(src['c'] * src['d']) + src['f'])
    d_r = f(powf(f(f(src['a'] * src['d']) + src['b']), src['g']) + src['e'])
    assert abs(float(d_l - d_r)) <= 1 / 512
    inv['d'] = d_l
    if inv['d'] > 0:
        inv['c'] = f(f(1) / src['c'])
        inv['f'] = f(-src['f'] / src['c'])
    k = powf(src['a'], -src['g'])
    inv['g'] = f(f(1) / src['g'])
    inv['a'] = k
    inv['b'] = f(-k * src['e'])
    inv['e'] = f(-src['b'] / src['a'])
    if f(f(inv['a'] * inv['d']) + inv['b']) < 0:
        inv['b'] = f(-inv['a'] * inv['d'])
    s = tf_eval(src, f(1))
    sign = f(-1) if s < 0 else f(1)
    s = f(s * sign)
    if s < inv['d']:
        inv['f'] = f(f(1) - f(sign * f(inv['c'] * s)))
    else:
        inv['e'] = f(f(1) - f(sign * powf(f(f(inv['a'] * s) + inv['b']), inv['g'])))
    return inv


D65toD50 = adapt_to_xyzd50(0.3127, 0.3290)
INV_D65_LMS = invert(kXYZD65_to_LMS)
INV_SRGB = invert(kSRGB_gamut)
INV_TF = tf_invert(kSRGB_tf)
PI_F = f(math.pi)


def lch_to_lab(l, c, h):
    rad = f(f(f(h) * PI_F) / f(180))
    return f(l), f(f(c) * cosf(rad)), f(f(c) * sinf(rad))


def oklab_to_xyzd50(l, a, b):
    lms = apply(kOklab_to_LMS, [f(l), f(a), f(b)])
    cubed = [f(f(x * x) * x) for x in lms]
    xyz65 = apply(INV_D65_LMS, cubed)
    return apply(D65toD50, xyz65)


def xyzd50_to_srgb(xyz):
    lin = apply(INV_SRGB, xyz)
    return [tf_eval(INV_TF, v) for v in lin], lin


def to_byte(v):
    # Sk4f_toL32 / appendConstantColor: (uint)(pin(v*255+0.5, 0, 255))
    t = f(f(f(v) * f(255)) + f(0.5))
    t = min(max(t, f(0)), f(255))
    return int(t)


def report(name, space, p0, p1, p2, alpha=1.0):
    p0, p1, p2, alpha = f(p0), f(p1), f(p2), f(alpha)
    if space == 'oklch':
        p0 = min(f(1), max(p0, f(0)))
        p1 = max(p1, f(0))
        l, a, b = lch_to_lab(p0, p1, p2)
    else:
        l, a, b = p0, p1, p2
    xyz = oklab_to_xyzd50(l, a, b)
    rgb, lin = xyzd50_to_srgb(xyz)
    bytes_ = [to_byte(v) for v in rgb] + [to_byte(alpha)]
    print(f"{name}")
    print(f"  oklab = ({float(l)!r}, {float(a)!r}, {float(b)!r})")
    print(f"  xyzd50 = {[float(x) for x in xyz]}")
    print(f"  linear sRGB = {[float(x) for x in lin]}")
    print(f"  SkColor4f = {[float(x) for x in rgb]} alpha {float(alpha)!r}")
    print(f"  8-bit RGBA (unpremul, clamped) = {bytes_}")
    return rgb, alpha


def premul_byte(c, a):
    prod = c * a + 128
    return (prod + (prod >> 8)) >> 8


def legacy_over(rgba, dst):
    r, g, b, a = rgba
    pm = [premul_byte(r, a), premul_byte(g, a), premul_byte(b, a)]
    inv = (255 - a) + 1
    return [((d * inv) >> 8) + c for d, c in zip(dst, pm)]


def section5():
    print('D65->D50 matrix', [[float(x) for x in r] for r in D65toD50])
    print('inv sRGB TF', {k: float(v) for k, v in INV_TF.items()})
    report('oklch(0.205 0 0)', 'oklch', 0.205, 0, 0)
    report('oklch(0.985 0 0)', 'oklch', 0.985, 0, 0)
    report('oklch(0.577 0.245 27.325)', 'oklch', 0.577, 0.245, 27.325)
    report('oklch(1 0 0 / 10%)', 'oklch', 1, 0, 0, 0.1)
    # color-mix(in oklab, oklch(0.205 0 0) 50%, transparent) -> oklab(0.205 0 0 / 0.5)
    l = f(0.205)
    p0 = f(float(l) + float(f(0) - l) * float(f(0.5)))  # Blend(float,float,double)
    alpha = f(1.0 + float(f(0) - f(1)) * 0.5)
    p0 = f(p0 / alpha)
    print('color-mix result oklab L =', float(p0), 'alpha =', float(alpha))
    report('color-mix(in oklab, oklch(0.205 0 0) 50%, transparent)', 'oklab', p0, 0, 0, alpha)
    rgba = [to_byte(v) for v in report('  (again)', 'oklab', p0, 0, 0, alpha)[0]] + [to_byte(alpha)]
    print('  legacy SrcOver on white:', legacy_over(rgba, [255, 255, 255]), 'on black:', legacy_over(rgba, [0, 0, 0]))
    rgba = [255, 255, 255, to_byte(f(0.1))]
    print('oklch(1 0 0/10%) legacy SrcOver on black:', legacy_over(rgba, [0, 0, 0]), 'on white', legacy_over(rgba, [255, 255, 255]))


# ---------------------------------------------------------------------------
# Section 7 follow-up: reverse path (legacy / color(srgb) / srgb-linear -> oklab/oklch),
# color-mix() emulation, skcms approximate transfer functions (srgb-linear paint path),
# and Skia CPU raster compositing helpers.
# ---------------------------------------------------------------------------
import struct

INV_D65toD50 = invert(D65toD50)          # skcms::Matrix3x3_apply_inverse(getXYDZ65toXYZD50matrix())
INV_OKLAB_TO_LMS = invert(kOklab_to_LMS)  # skcms::Matrix3x3_apply_inverse(kOklab_to_LMS)
kLinear_tf = dict(g=f(1), a=f(1), b=f(0), c=f(0), d=f(0), e=f(0), f=f(0))


def powExt(x, p):
    # color_conversions.cc:30-36 (powf)
    return f(-powf(f(-x), p)) if x < 0 else powf(x, p)


def srgb_to_xyzd50(r, g, b):
    # gfx::SRGBToXYZD50: skcms::TransferFunction_apply(kSRGB) then Matrix3x3_apply(kSRGB)
    c = [tf_eval(kSRGB_tf, v) for v in (r, g, b)]
    return apply(kSRGB_gamut, c)


def srgb_linear_to_xyzd50(r, g, b):
    c = [tf_eval(kLinear_tf, v) for v in (r, g, b)]   # identity
    return apply(kSRGB_gamut, c)


def xyzd50_to_oklab(xyz):
    # gfx::XYZD50ToOklab (color_conversions.cc:251-263)
    x65 = apply(INV_D65toD50, xyz)
    lms_cubed = apply(kXYZD65_to_LMS, x65)
    third = f(f(1) / f(3))
    lms = [powExt(v, third) for v in lms_cubed]
    return apply(INV_OKLAB_TO_LMS, lms)


def lab_to_lch(l, a, b):
    # gfx::LabToLch: sqrt(a*a+b*b) (float), RadToDeg(atan2f(b,a)) = rad*180/pi_f
    c = f(math.sqrt(float(f(f(a * a) + f(b * b)))))
    rad = f(math.atan2(float(b), float(a)))
    h = f(f(rad * f(180)) / PI_F)
    return f(l), c, h


def angle_unit(h):
    # AngleToUnitCircleDegrees: fmod(fmod(angle, 360.f) + 360.f, 360.f)
    return f(math.fmod(float(f(f(math.fmod(float(h), 360.0)) + f(360))), 360.0))


class Col:
    def __init__(self, space, p, alpha=1.0, none=(False, False, False), alpha_none=False):
        self.space = space
        self.p = [f(v) for v in p]
        self.alpha = f(alpha)
        self.none = list(none)
        self.alpha_none = alpha_none

    def __repr__(self):
        vals = ' '.join('none' if n else '%.6g' % float(v) for v, n in zip(self.p, self.none))
        return '%s(%s / %.6g)' % (self.space, vals, float(self.alpha))


def legacy(r, g, b, a=255):
    # Color(RGBA32): params are 0..255 floats, alpha = a/255.f
    return Col('rgb', [r, g, b], f(f(a) / f(255)))


def to_xyzd50(c):
    s, (p0, p1, p2) = c.space, c.p
    if s == 'rgb':
        return srgb_to_xyzd50(f(p0 / 255.0), f(p1 / 255.0), f(p2 / 255.0))  # SRGBLegacyToSRGB: r / 255.0
    if s == 'srgb':
        return srgb_to_xyzd50(p0, p1, p2)
    if s == 'srgb-linear':
        return srgb_linear_to_xyzd50(p0, p1, p2)
    if s == 'oklab':
        return oklab_to_xyzd50(p0, p1, p2)
    if s == 'oklch':
        return oklab_to_xyzd50(*lch_to_lab(p0, p1, p2))
    raise ValueError(s)


def to_srgb(c):
    # Color::ToSRGB(false)
    s, (p0, p1, p2) = c.space, c.p
    if s == 'srgb':
        return [p0, p1, p2]
    if s == 'rgb':
        return [f(p0 / 255.0), f(p1 / 255.0), f(p2 / 255.0)]
    if s == 'srgb-linear':
        inv = skcms_tf_invert(kSRGB_tf)        # skcms's own (approximate) invert + eval
        return [skcms_tf_eval(inv, v) for v in (p0, p1, p2)]
    return xyzd50_to_srgb(to_xyzd50(c))[0]


def convert(c, dst):
    # Color::ConvertToColorSpace (resolve_missing_components = true)
    if c.space == dst:
        return Col(c.space, c.p, c.alpha, c.none, c.alpha_none)
    p = [f(0) if n else v for v, n in zip(c.p, c.none)]
    src = Col(c.space, p, c.alpha)
    none = [False, False, False]
    if dst == 'srgb':
        out = to_srgb(src)
    elif dst == 'srgb-linear':
        out = apply(INV_SRGB, to_xyzd50(src))   # TransferFunction_apply_inverse(kLinear) is identity
    elif dst == 'oklab':
        out = list(lch_to_lab(*p)) if c.space == 'oklch' else xyzd50_to_oklab(to_xyzd50(src))
    elif dst == 'oklch':
        if c.space == 'oklab':
            out = list(lab_to_lch(*p))       # NOTE: no AngleToUnitCircleDegrees here (color.cc:771-773)
        else:
            out = list(lab_to_lch(*xyzd50_to_oklab(to_xyzd50(src))))
            out[2] = angle_unit(out[2])
        if out[1] <= f(0.02):                # kAchromaticChromaThreshold
            none[2] = True
    else:
        raise ValueError(dst)
    return Col(dst, out, c.alpha, none, c.alpha_none)


def analogous(c, dst):
    rgbxyz = ('rgb', 'srgb', 'srgb-linear')
    if c.space in rgbxyz and dst in rgbxyz:
        return list(c.none)
    if c.space in ('oklab', 'oklch') and dst in ('oklab', 'oklch'):
        return [c.none[0], False, False]
    return [False, False, False]


def convert_for_interp(c, dst):
    if c.space == dst:
        return Col(c.space, c.p, c.alpha, c.none, c.alpha_none)
    an = analogous(c, dst)
    out = convert(c, dst)
    for i in range(3):
        if an[i]:
            out.p[i] = f(0)
            out.none[i] = True
    return out


def fblend(a, b, p):
    # blink::Blend(float from, float to, double progress)
    return f(float(a) + float(f(b - a)) * float(p))


def hue_interp(v1, v2, p, method='shorter'):
    v1, v2 = f(v1), f(v2)
    diff = f(v2 - v1)
    if method == 'shorter':
        if diff > 180:
            v1 = f(v1 + f(360))
        elif diff < -180:
            v2 = f(v2 + f(360))
    return angle_unit(fblend(v1, v2, p))


def normalize_percentages(p1, p2):
    # css_color_mix_value.cc:14-60 (double); p1/p2 are percents or None
    q1 = 0.5 if p1 is None else min(max(p1, 0.0), 100.0) / 100.0
    q2 = 0.5 if p2 is None else min(max(p2, 0.0), 100.0) / 100.0
    if p1 is not None and p2 is None:
        q2 = 1.0 - q1
    elif p2 is not None and p1 is None:
        q1 = 1.0 - q2
    alpha_mult = 1.0
    scale = q1 + q2
    if scale != 0.0:
        q1 /= scale
        q2 /= scale
        if scale <= 1.0:
            alpha_mult = scale
    mix = q2
    if q1 == 0.0:
        mix = 1.0
    return mix, alpha_mult


def color_mix(space, c1, p1, c2, p2, method='shorter'):
    mix, am = normalize_percentages(p1, p2)
    pct, am = f(mix), f(am)                   # narrowed to float by Color::FromColorMix
    a = convert_for_interp(c1, space)
    b = convert_for_interp(c2, space)
    for i in range(3):                       # SubstituteMissingParameters
        if a.none[i] and not b.none[i]:
            a.p[i] = b.p[i]; a.none[i] = False
        elif b.none[i] and not a.none[i]:
            b.p[i] = a.p[i]; b.none[i] = False
    hue_idx = 2 if space == 'oklch' else None

    def premul(c):
        al = c.alpha
        for i in range(3):
            if i != hue_idx:
                c.p[i] = f(c.p[i] * al)
        c.alpha = f(1)
        return al

    a1, a2 = premul(a), premul(b)
    out = []
    for i in range(3):
        if a.none[i] and b.none[i]:
            out.append(None)
        elif i == hue_idx:
            out.append(hue_interp(a.p[i], b.p[i], pct, method))
        else:
            out.append(fblend(a.p[i], b.p[i], pct))
    alpha = fblend(a1, a2, pct)
    res = Col(space, [f(0) if v is None else v for v in out], alpha, [v is None for v in out])
    if space in ('oklab', 'oklch'):
        res.p[0] = min(f(1), max(res.p[0], f(0)))
    if space == 'oklch':
        res.p[1] = max(res.p[1], f(0))
    if res.alpha != 0:                       # UnpremultiplyColor
        for i in range(3):
            if i != hue_idx:
                res.p[i] = f(res.p[i] / res.alpha)
    res.alpha = f(res.alpha * am)
    return res


# ---- skcms approximate math (used ONLY by Color::ToSRGB for kSRGBLinear) ----
def _bits(x):
    return struct.unpack('<i', struct.pack('<f', float(x)))[0]


def _from_bits(i):
    return f(struct.unpack('<f', struct.pack('<i', i))[0])


def log2f_(x):
    bits = _bits(x)
    e = f(f(bits) * f(1.0 / (1 << 23)))
    m = _from_bits((bits & 0x007fffff) | 0x3f000000)
    return f(f(f(e - f(124.225514990)) - f(f(1.498030302) * m)) - f(f(1.725879990) / f(f(0.3520887068) + m)))


def floorf_(x):
    rt = f(int(x))
    return f(rt - 1) if rt > x else rt


def exp2f_(x):
    x = f(x)
    if x > 128:
        return f(math.inf)
    if x < -127:
        return f(0)
    fract = f(x - floorf_(x))
    inner = f(f(f(x + f(121.274057500)) - f(f(1.490129070) * fract)) + f(f(27.728023300) / f(f(4.84252568) - fract)))
    fbits = f(f(1 << 23) * inner)
    if fbits >= f(2147483647):
        return f(math.inf)
    if fbits < 0:
        return f(0)
    return _from_bits(int(fbits))


def powf_(x, y):
    x = f(x)
    if x <= 0:
        return f(0)
    if x == 1:
        return f(1)
    return exp2f_(f(log2f_(x) * f(y)))


def skcms_tf_eval(tf, x):
    x = f(x)
    sign = f(-1) if x < 0 else f(1)
    x = f(x * sign)
    if x < tf['d']:
        return f(sign * f(f(tf['c'] * x) + tf['f']))
    return f(sign * f(powf_(f(f(tf['a'] * x) + tf['b']), tf['g']) + tf['e']))


def skcms_tf_invert(src):
    inv = dict(g=f(0), a=f(0), b=f(0), c=f(0), d=f(0), e=f(0), f=f(0))
    inv['d'] = f(f(src['c'] * src['d']) + src['f'])
    if inv['d'] > 0:
        inv['c'] = f(f(1) / src['c'])
        inv['f'] = f(-src['f'] / src['c'])
    k = powf_(src['a'], f(-src['g']))
    inv['g'] = f(f(1) / src['g'])
    inv['a'] = k
    inv['b'] = f(-k * src['e'])
    inv['e'] = f(-src['b'] / src['a'])
    if f(f(inv['a'] * inv['d']) + inv['b']) < 0:
        inv['b'] = f(-inv['a'] * inv['d'])
    s = skcms_tf_eval(src, f(1))
    sign = f(-1) if s < 0 else f(1)
    s = f(s * sign)
    if s < inv['d']:
        inv['f'] = f(f(1) - f(f(sign * inv['c']) * s))
    else:
        inv['e'] = f(f(1) - f(sign * powf_(f(f(inv['a'] * s) + inv['b']), inv['g'])))
    return inv


# ---- Skia CPU raster (--disable-gpu) ----
def fits_in_bytes(rgb):
    return all(f(0) <= v <= f(1) for v in rgb)


def composite(rgb, alpha, dst):
    """Solid SrcOver fill over an opaque 8-bit dst, per SkBlitter::UseLegacyBlitter."""
    if fits_in_bytes(rgb):
        r, g, b, a = [to_byte(v) for v in rgb] + [to_byte(alpha)]
        if a == 255:
            return [r, g, b], 'legacy-opaque'
        return legacy_over([r, g, b, a], dst), 'legacy-ARGB32'
    pm = [min(max(f(v * alpha), f(0)), f(1)) for v in rgb]
    if alpha == 1:
        return [int(f(f(v * f(255)) + f(0.5))) for v in pm], 'SkRP uniform/memset'
    out = []
    for s, d in zip(pm, dst):
        dv = f(f(d) * f(1 / 255.0))
        v = f(f(dv * f(f(1) - alpha)) + s)
        out.append(int(np.rint(min(max(f(0), f(v * f(255))), f(255)))))
    return out, 'SkRP highp'


def paint_report(label, c):
    rgb = to_srgb(c)
    print(label)
    print('  computed value :', c)
    print('  SkColor4f      :', [float(v) for v in rgb], 'alpha', float(c.alpha))
    print('  8-bit unpremul :', [to_byte(v) for v in rgb] + [to_byte(c.alpha)])
    for name, dst in (('white', [255, 255, 255]), ('black', [0, 0, 0])):
        px, path = composite(rgb, c.alpha, dst)
        print('  over %-5s     : %s  (%s)' % (name, px, path))
    return rgb


def followup():
    print('\n=== Section 7 follow-up ===')
    white = legacy(255, 255, 255)
    transparent = legacy(0, 0, 0, 0)
    blue500 = legacy(0x3b, 0x82, 0xf6)
    print('white -> oklab', convert(white, 'oklab'), ' -> oklch', convert(white, 'oklch'))
    print('#3b82f6 -> oklab', convert(blue500, 'oklab'), ' -> oklch', convert(blue500, 'oklch'))
    paint_report('color-mix(in oklab, white 10%, transparent)',
                 color_mix('oklab', white, 10, transparent, None))
    paint_report('color-mix(in oklab, #3b82f6 50%, transparent)',
                 color_mix('oklab', blue500, 50, transparent, None))
    paint_report('color-mix(in oklch, oklch(0.7 0.1 200), white)',
                 color_mix('oklch', Col('oklch', [0.7, 0.1, 200]), None, white, None))
    paint_report('oklch(0.7 0.1 200) alone', Col('oklch', [0.7, 0.1, 200]))
    m = color_mix('srgb', legacy(255, 0, 0), 30, legacy(0, 0, 255), None)
    paint_report('color-mix(in srgb, red 30%, blue)', m)
    print('  r*255f =', float(f(m.p[0] * f(255))), ' b*255f =', float(f(m.p[2] * f(255))))
    paint_report('color-mix(in srgb-linear, red 30%, blue)',
                 color_mix('srgb-linear', legacy(255, 0, 0), 30, legacy(0, 0, 255), None))
    inv = skcms_tf_invert(kSRGB_tf)
    print('skcms (approx) inverse sRGB tf:', {k: float(v) for k, v in inv.items()})


if __name__ == '__main__':
    section5()
    followup()
