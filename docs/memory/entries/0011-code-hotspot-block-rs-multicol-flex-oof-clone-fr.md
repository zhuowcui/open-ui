---
id: 0011
title: Code hotspot: block.rs multicol/flex/OOF/clone fragmentation
tags: hotspot, layout, sp13, block-rs
status: active
created: 2026-07-08
updated: 2026-07-08
refs: bindings/rust/openui-layout/src/block.rs
---

bindings/rust/openui-layout/src/block.rs is the main multicol/block/flex fragmentation impl. Key regions (line numbers approximate, drift over time — grep to confirm): fragment_visual_block_bottom/top ~4184; first-pass multicol sizing/balancing ~4724-5155; fitting child path + visual-overflow continuation ~6330-6890; fragmenting child path ~6893-7855; ColumnBox wrapping ~8130-8215; OOF handling ~8488-8780. Recent SP13 fixes (flex-container gates, direct-abspos multicol continuation, clone visual-overflow) live in the fitting/fragmenting child paths.
