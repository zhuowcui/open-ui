---
title: Components
description: shadcn/ui components for Open UI, and how each is measured.
---

The components track ports [shadcn/ui](https://ui.shadcn.com/) to Open UI
under the working name `openui-components`, in
[`components/`](https://github.com/zhuowcui/open-ui/tree/main/components).
Each fixture is rendered by shadcn/ui in pinned Chromium 147 and by Open UI,
at the four census profiles, and compared byte for byte.

| Component | On `main` | Exact |
| --- | --- | --- |
| separator | yes | 8 of 16: every light comparison; dark not remeasured since a color fix |
| button, badge, card, label | yes, static styles | not yet: text differs (see [Fidelity](../fidelity/#known-failures)) |

alpha.1 aims at ten components (button, input, label, card, badge,
separator, checkbox, switch, tabs and dialog) in light and dark themes, each
with its measured result published. Exactness is the target, not the release
condition. Interactive components wait for typed style rules with state
variants (RFC 0001) and the primitives in
[RFC 0003](https://github.com/zhuowcui/open-ui/blob/main/docs/alpha/rfcs/0003-component-primitives.md).

Results and their commits are in the
[components track log](https://github.com/zhuowcui/open-ui/blob/main/docs/progress/tracks/components.md).
