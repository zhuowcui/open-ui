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
public API: `Document::body().bounding_rect()` forces a layout read,
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
