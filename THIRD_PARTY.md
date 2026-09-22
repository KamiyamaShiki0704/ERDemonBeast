# Third-party references

- Runtime bindings: [fromsoftware-rs](https://github.com/KamiyamaShiki0704/fromsoftware-rs/tree/02fa5681e27fd2ecd9da79d34aef9fae96805539), pinned in Cargo.lock; MIT OR Apache-2.0.
- Summon lifecycle/retirement framework: [ER_Summon](https://github.com/KamiyamaShiki0704/ER_Summon/tree/01491edfa709545786d0fe795ab2fb2458dca4e1), fixed revision `01491edfa709545786d0fe795ab2fb2458dca4e1`, reused under MIT. Exact source, binary fixtures, license and hashes are in `src/vendor/summon/`; integration notes are in its PROVENANCE.md. No sibling project is modified or linked by filesystem path. The creation layout follows the same revision; our request-ownership checks remain project-specific.
- Debug control interface reference: [AlpinDale/er_enemy_control](https://github.com/AlpinDale/er_enemy_control/tree/8056f10c0864e3e10d6c20cc85827676fac36ab4), Apache-2.0. Interface semantics were cross-checked against the target binary; this project has its own state machine and recovery implementation.

Other Rust dependencies and exact versions are recorded in Cargo.lock. This is a local development prototype; prepare the full dependency license bundle before external distribution. No game assets are included.
