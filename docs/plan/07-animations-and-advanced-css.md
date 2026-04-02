# Sub-Project 7: Animations & Advanced CSS

> CSS animations, transitions, transforms, and compositor-thread rendering at 60fps.

## Objective

Enable Chromium's animation and transition systems through the C API. Animations run on the compositor thread (via cc/) for jank-free rendering even when the main thread is busy. This includes CSS transitions, CSS animations (@keyframes), and scroll-driven animations.

## Tasks

### Phase A: CSS Transitions
1. Property transition support via C API (`oui_element_set_transition()`)
2. Transition timing functions (ease, linear, ease-in-out, cubic-bezier)
3. Compositor-thread transitions for transform, opacity, filter
4. Main-thread transitions for layout-affecting properties

### Phase B: CSS Animations
5. @keyframes-equivalent animation definition via C API
6. Animation properties (duration, delay, iteration-count, direction, fill-mode)
7. Compositor-thread animations for transform/opacity
8. Animation events (start, end, iteration) surfaced through C API

### Phase C: Scroll Animations
9. Scroll-driven animations (scroll-timeline equivalent)
10. Smooth scrolling with compositor-thread physics
11. Scroll snap behavior

### Phase D: Transform & 3D
12. 3D transforms (perspective, rotateX/Y/Z, translate3d)
13. Transform-style: preserve-3d
14. Backface-visibility

### Phase E: Performance & Verification
15. Verify 60fps for 100 animated layers
16. Verify compositor-thread independence (block main thread → animations continue)
17. Frame timing accuracy (< 2ms variance)
18. Animation pixel-comparison tests

## Deliverables

| Deliverable | Description |
|---|---|
| Animation C API | `oui_element_set_transition()`, `oui_animation_create()`, etc. |
| `examples/animation_demo.c` | Animated UI demo |
| `examples/scroll_animation.c` | Scroll-driven animation demo |
| `tests/animation/` | Animation correctness and timing tests |
| `benchmarks/animation/` | 60fps verification benchmarks |

## Success Criteria

- [ ] CSS transitions animate smoothly via C API
- [ ] Compositor-thread animations run at 60fps while main thread blocked
- [ ] Scroll animations track scroll position correctly
- [ ] 3D transforms render with correct perspective
- [ ] Frame time variance < 2ms at 60fps
