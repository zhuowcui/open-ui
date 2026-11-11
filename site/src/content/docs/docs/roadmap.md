---
title: Roadmap
description: What is in the first alpha and what comes next.
---

Div is in its first alpha. Dates are not promised, and the plan can
change.

## alpha.1: Windows and Linux

**Ready to try**

- The `openui` CLI: `new`, `dev` (rebuild and restart on save) and `build`.
- Windows, menus, file dialogs, clipboard, async tasks and an HTTP client.
- Headless UI tests with pixel snapshots.
- A component library based on shadcn/ui designs, started: button, badge,
  card, label and separator.
- Modern CSS colors: `oklch()`, `oklab()` and `color-mix()`.

**In progress**

- Stylesheets and classes, state variants such as `:hover`, and theme
  tokens.
- More components, and dialogs drawn above the page.
- Text that matches the browser to the pixel with web fonts such as Geist
  and Inter.
- A tutorial and an API reference on this site.

## alpha.2

- macOS.
- Select, tooltip, toast and other overlays.
- Live reload of markup and styles.
- Notifications and installers.

Not promised in alpha: a stable API, signed installers or auto-update.

The detailed plan is
[`docs/alpha/plan.md`](https://github.com/zhuowcui/open-ui/blob/main/docs/alpha/plan.md)
in the repository.
