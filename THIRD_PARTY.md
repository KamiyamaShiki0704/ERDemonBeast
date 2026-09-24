# Third-party components

## Summon

Character lifecycle and retirement code derives from
[ER_Summon](https://github.com/KamiyamaShiki0704/ER_Summon/tree/01491edfa709545786d0fe795ab2fb2458dca4e1),
revision `01491edfa709545786d0fe795ab2fb2458dca4e1`.

Copyright (c) 2025 Vincent Swarte. Licensed under MIT; the original notice is
included at [src/vendor/summon/LICENSE-MIT](src/vendor/summon/LICENSE-MIT).
The vendored character lifecycle code and `src/retirement.rs` retain this
attribution. Native instruction signatures are required by the runtime's
version checks. Source information is documented in
[src/vendor/summon/PROVENANCE.md](src/vendor/summon/PROVENANCE.md).

## Runtime bindings

[fromsoftware-rs](https://github.com/KamiyamaShiki0704/fromsoftware-rs/tree/02fa5681e27fd2ecd9da79d34aef9fae96805539)
provides the Elden Ring bindings. Its revision is pinned in `Cargo.toml` and
`Cargo.lock`; the upstream project is licensed under MIT OR Apache-2.0.

## Interface reference

[er_enemy_control](https://github.com/AlpinDale/er_enemy_control/tree/8056f10c0864e3e10d6c20cc85827676fac36ab4)
(Apache-2.0) was used as a reference for debug control interface semantics.
ER Demon Beast implements its own form selection, state machine and recovery
logic.

## Other dependencies and assets

Additional Rust dependencies and their exact versions are listed in `Cargo.lock`.
They retain their respective licenses. This repository does not distribute their
source archives or compiled third-party libraries.

The MIT license for this project does not grant rights to Elden Ring or
Nightreign models, animations, original scripts or other game assets. Those
assets are not included in this repository.
