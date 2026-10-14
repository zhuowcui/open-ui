# Native style ownership and property removal

Native application styles retain their typed source values. Parent fonts,
root fonts, viewport changes, currentcolor and active animations are resolved
through the shared Engine cascade.

`Engine::remove_property(handle, property)` and
`Element::remove_property(property)` return `true` when the requested canonical
fields had an authored owner, and `false` without work when they did not. A
removed longhand stays removed from older shorthands. Removing a shorthand
removes its remaining canonical fields; independent declarations remain.

Native constructor defaults are permanent source inputs. For example, removing
an authored `display` restores a native div's block default, a table's table
default, or a list item's list-item default. Removing a width restores the
source width, normally auto. Raw `Engine::create_element` retains its existing
initial-style defaults.

Animations can continue masking a removed source value. Cancellation or an
unfilled phase reveals the remaining authored declarations over the permanent
source. The transient animation cache never becomes that source. Transitions
write their destination through the same canonical ownership route.

Renderer compatibility and explicitly installed derived styles are computed
snapshot inputs. They retain their existing inheritance policy, and removing a
later authored override restores that explicit snapshot input. Snapshot creation
uses the unanimated cascade rather than the currently displayed animation.

`oui_element_remove_property(element, property, &removed)` exposes the same
operation to C and C++. Its mandatory byte output is written only on success:
`1` means ownership was removed, and `0` means the property had no remaining
authored owner. Errors preserve the output. Handles and calls use the creating
thread; removal is allowed from event callbacks after internal borrows end.

Retained raw border-radius source is preserved;
actual-box paint/hit/scene consumers and full authored elliptical radius transport
have separate qualification gates. Source and typed guard review do not qualify
runtime behavior or Chromium pixels.
