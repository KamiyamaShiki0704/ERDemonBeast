# Third-party references

- Runtime bindings: [fromsoftware-rs](https://github.com/KamiyamaShiki0704/fromsoftware-rs/tree/02fa5681e27fd2ecd9da79d34aef9fae96805539), pinned in Cargo.lock; MIT OR Apache-2.0.
- Existing spawn workflow reference: local Summon project at commit `20c259d0c8a2597b5421d88c2565aa0dc89fb7bb` and its working-tree source; MIT OR Apache-2.0. No sibling project is modified or linked by filesystem path.
- Debug control interface reference: [AlpinDale/er_enemy_control](https://github.com/AlpinDale/er_enemy_control/tree/8056f10c0864e3e10d6c20cc85827676fac36ab4), Apache-2.0. Interface semantics were cross-checked against the target binary; this project has its own state machine and recovery implementation.

Other Rust dependencies and exact versions are recorded in Cargo.lock. This is a local development prototype; prepare the full dependency license bundle before external distribution. No game assets are included.
