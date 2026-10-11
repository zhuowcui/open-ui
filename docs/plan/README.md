# Historical implementation plans

The documents in this directory describe the original SP1–SP9 Chromium/Blink
extraction strategy. They are retained as engineering history and are not the
supported product roadmap.

Open UI v0.2 converged on one pure-Rust renderer. The active contracts are:

- [`../v02/roadmap.md`](../v02/roadmap.md) — remaining v0.2 qualification and
  post-v0.2 platform work;
- [`../adr/004-openui-v02-linux-product.md`](../adr/004-openui-v02-linux-product.md)
  — architecture decision;
- [`../progress/current-status.md`](../progress/current-status.md) — verified
  repository status and blockers;
- [`../v02/release.md`](../v02/release.md) — final release decision matrix.

Open UI does not execute JavaScript or embed V8. Application interaction runs
in native Rust through public `Document` and `Element` methods and Rust event
callbacks. When an application needs browser-like element behavior, the shared
engine must expose it through the public native Rust API. See the
[native interaction contract](../v02/supported-platforms.md#native-interaction-api).

Do not implement new features in the historical Blink backend to satisfy an
active v0.2 requirement.
