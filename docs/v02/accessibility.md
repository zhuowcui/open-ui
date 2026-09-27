# Accessibility contract

Open UI v0.2 produces its accessibility tree from the same retained document,
control state, focus state, layout, and dirty generations used for rendering.
The engine exposes AccessKit `TreeUpdate` snapshots: the first update is a full
tree and later updates replace only semantic nodes whose public state changed.
An unchanged document produces an empty node update.

Each authored engine node has a stable accessibility ID derived from its
generation-checked node handle. Removing and reusing a document slot therefore
cannot make an old accessibility ID address the new node. Editable controls add
a private text-run child containing UTF-8 character lengths and the current
selection. Bounds are transformed device-independent coordinates derived from
the retained fragment and hit-test data.

Default roles and states are inferred for the v0.2 controls, tables, images,
details/summary, forms, and text. Typed Rust and C APIs can override role, name,
description, value, live-region mode, hidden state, and labelled-by,
described-by, controls, and details relations. Relation targets must be live
nodes in the same document.

Accessibility actions use the ordinary interaction path. Click, focus, blur,
value adjustment, expansion, text replacement, selection, and scrolling mutate
the same engine state as pointer and keyboard input. Framework and C listeners
receive the corresponding click, focus, blur, input, and change events without
an internal engine borrow being held across an application callback.

The platform preference for reduced motion is retained by the engine and is
included in accessibility update metadata for native consumers. The Linux
platform crate owns the AccessKit winit/Unix adapter and forwards its action
requests into this API; AT-SPI is not accessed from headless builds.

C applications can request an owned `OuiAccessibilitySnapshot` from the same
engine tree. The snapshot exposes every node's stable ID, role and role name,
label, description, value, placeholder, bounds, state flags, supported core
actions, ordered children, labelled-by, described-by, controls, and details
relations. Control state includes selected, expanded, toggled, live region,
numeric values, scroll offsets, and text selection/character lengths. Its
metadata includes focus, generation, and reduced-motion preference. Passing a
previous snapshot from the same document returns changed and removed IDs;
each snapshot still contains the full current tree. A snapshot remains readable
after its document is destroyed and must be destroyed on its owning thread.
This API does not consume the Rust/AccessKit incremental update stream.

The v0.2 accessibility scope covers the core interactive controls and their
text editing behavior. Rich hypertext navigation beyond those controls remains
deferred.
