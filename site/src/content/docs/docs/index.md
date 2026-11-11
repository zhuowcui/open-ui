---
title: Docs
description: Where the Open UI documentation stands.
---

The guide is not written yet. A docs track will add a tutorial, core
concepts, a "coming from the web" guide, the API reference and a CSS support
matrix generated from the census, as planned in the
[alpha plan](https://github.com/zhuowcui/open-ui/blob/main/docs/alpha/plan.md).

Until then:

- [Getting started](../getting-started/) covers the CLI and the default template.
- [Fidelity](../fidelity/) covers how rendering is measured.
- The design documents are the RFCs in
  [`docs/alpha/rfcs/`](https://github.com/zhuowcui/open-ui/tree/main/docs/alpha/rfcs):
  typed style rules (0001), the CLI and app layout (0002) and component
  primitives (0003).
- The Rust workspace is in
  [`bindings/rust`](https://github.com/zhuowcui/open-ui/tree/main/bindings/rust);
  `cargo doc --open` there builds the API docs locally.
