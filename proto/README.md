# Vendored arduino-cli protos

This directory holds the vendored `arduino-cli` gRPC `*.proto` files. It is
empty until the gRPC milestone — [build.rs](../build.rs) skips proto codegen
while no `.proto` files are present, so the crate builds without them.

Source of truth for what goes here: `ArduinoCoreService` from the `arduino-cli`
repo (see `.agents/docs/21-06_01.arduino-helper-design.md`).
