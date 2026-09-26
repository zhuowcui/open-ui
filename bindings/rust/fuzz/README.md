# Open UI fuzz targets

This isolated `cargo-fuzz` workspace covers the five stateful v0.2 attack
surfaces: raw C style values, generation-checked tree mutations, pointer event
sequences, resource registration, and animation timing. It is excluded from
the release workspace so `libfuzzer-sys` never enters an Open UI runtime
artifact.

Install `cargo-fuzz`, select the pinned nightly from the hardening workflow,
then run a target from this directory:

```sh
cargo +nightly-2026-09-01 fuzz run tree_mutations
```

The nested `Cargo.lock` pins `libfuzzer-sys` and all transitive dependencies.
Crashes and generated corpora are local artifacts and are ignored by Git.
