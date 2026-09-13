# Chromium 147 typography schema

Open UI's canonical public typography surface is generated from
[`property-schema.csv`](../../bindings/rust/openui-style/property-schema.csv).
Property IDs 1–46 remain unchanged; the Chromium 147 closure occupies IDs
47–125.

The inventory was audited against Chromium `147.0.7727.24` at revision
`09d377d9438dc95267369f74a073acd81bdde38f`. The source inputs were:

- `css_properties.json5` SHA-256
  `c83f99d7c0513f3b79127d247796a9cc155a6c57121c29e04b11a1871b66f814`;
- `runtime_enabled_features.json5` SHA-256
  `1eccbc38eac157120e569dd6d6900f2c5a5173bc2f55d143376bf1ad6f6864e4`.

The audit includes all 77 default-enabled author properties selected from CSS
Fonts, CSS Text, CSS Text Decorations, CSS Writing Modes, and CSS Ruby. It also
keeps `line-clamp`, `block-ellipsis`, `ruby-overhang`, and
`font-synthesis-position`, plus `hanging-punctuation`, in the typed surface because the renderer contract
explicitly requires those values even where Chromium gates their syntax.

The generator emits the same metadata into Rust property IDs, `Style`
builders, `Element` setters, `view!` literal validation, C property IDs and
tagged-value expectations, and the generated property reference. Inherited
values are copied at pseudo/anonymous-box boundaries, while invalidation and
interpolation classes remain part of each property's canonical row.

Compound values are immutable typed builders. The `font`, `font-variant`,
`font-synthesis`, `white-space`, `text-wrap`, `text-decoration`, `text-emphasis`,
and `text-box` shorthands are represented by one declaration payload and are
applied atomically. `PseudoStyleTarget` covers `::first-line`, `::first-letter`,
`::marker`, and `::placeholder`; `LanguageTag` is the validated BCP 47 input for
shaping and hyphenation.

For C callers, `oui_style_value_parse` validates a length-delimited literal
against a concrete property ID and returns the exact required tagged value.
Compound payload handles have an explicit lifetime and may be destroyed as
soon as `oui_element_set_property` returns because submission retains an owned
copy.
