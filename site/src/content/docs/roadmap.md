---
title: Roadmap
description: What alpha.1 and alpha.2 contain, from the alpha plan.
---

This summarizes the [alpha plan](https://github.com/zhuowcui/open-ui/blob/main/docs/alpha/plan.md), which is
the source of truth and still a draft. Dates are not promised.

## alpha.1: Windows and Linux

- `openui new`, `openui dev` (rebuild and restart) and `openui build`.
  **Merged.**
- App services: several windows, an app menu, file dialogs, clipboard, an
  async runtime and an HTTP client. **Merged.**
- Headless UI tests and snapshots, and the default template. **Merged.**
- `oklch()`, `oklab()` and `color-mix()` as Chromium resolves them.
  **Merged.**
- Typed style rules (`style!`, `css!`, `stylesheet!`), state variants,
  theme tokens and media conditions. **In progress**: the first step is open
  as [#109](https://github.com/zhuowcui/open-ui/pull/109).
- A top layer for one overlay, the dialog. **Planned.**
- Real-font text measured and repaired for Geist and Inter. **Planned.**
- Ten shadcn/ui components, each with its measured result. **Five static ones merged** (separator, button, badge, card, label).
- A docs site with getting started, a tutorial and a CSS support matrix.
  **This site is the start.**

Not promised in alpha: API stability, Windows accessibility behavior, signed
installers or auto-update.

## alpha.2

macOS, menus, select, tooltip, toast and other overlays, live reload of
styles and markup, notifications and installers.

## Open decisions

The product name is deferred; "Open UI" is a working name. macOS needs Mac
hardware or hosted runners and a macOS Chromium oracle before it can start.
