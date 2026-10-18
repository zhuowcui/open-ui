# Adjoining floats native final-state admission

The Chromium source for `wpt/css2_floats/adjoining-floats-dynamic` first reads
`document.body.offsetTop` to force layout, then sets the width of an element
found by ID to `50px`. The pinned Acorn audit records exactly those two
ordered operations. Open UI does not execute the script. Its renderer fixture
performs the layout barrier and typed width mutation through the retained
Engine.

The [clean pending-candidate matrix](generated/pending-mutation-candidates-v5.json)
at `814a2005` found this case exact at all four required Chromium profiles,
with zero differing pixels and zero errors. The result uses the same pinned
Chromium oracle and zero-tolerance comparator as the original census. The
other 35 pending cases did not become exact at all four profiles.

A consuming Rust application can perform this case's interaction through the
public API: `document.body().bounding_rect()` forces a layout read,
`Document::element_by_id` finds the attached target, and `Element::set_width`
applies the typed mutation. The
`native_layout_read_then_id_style_mutation_updates_the_same_document`
conformance test exercises that path through `openui`, rather than through a
test-only Engine handle.

[`expanded-v18.json`](../../tools/qualification/manifests/expanded-v18.json)
versions this case as the 201st native final-state addition. It retains all
200 earlier additions. Three of those earlier additions currently differ from
Chromium at one profile each, so the expanded release gate still fails. This
admission does not change the original 5,731-case inventory, archived Open UI
images, or Chromium oracle bytes.

The complete clean [v17 expanded requalification](generated/expanded-requalification-v17.json)
at `9534c9f6` is 22,040/23,728 exact, with 1,688 differences and zero errors.
This case adds four exact profile comparisons. All 23,724 comparisons from the
previous expanded run have identical decoded Open UI and Chromium hashes,
statuses, and diff signatures. The 201 additions now include 198 that are
exact at all four profiles and the same three earlier failures. The original
[v31 census](generated/four-profile-census-v31.json) remains
21,239/22,924 exact; the [focused and primitive gates](generated/focused-primitive-raster-v32.json)
remain 640/640 and 960/960 exact. The other 35
[AST-lowered candidates](generated/pending-mutation-candidates-v6.json)
remain pending.

The new contract differs from the previous one only in the expanded manifest
path, hash, and case count. Cache indexes were carried forward only after
checking that all other contract fields, the renderer binary, and every
mutation sequence were unchanged. SHA-256 checks verified 11,284 unique
Chromium PNGs and 17,269 unique prior result PNGs. Reindexing changed no PNG
bytes, and the clean matrix runner still checked every current fixture and
profile identity before using a cached result.
