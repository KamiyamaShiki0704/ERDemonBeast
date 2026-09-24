# Summon source attribution

Upstream: [ER_Summon](https://github.com/KamiyamaShiki0704/ER_Summon/tree/01491edfa709545786d0fe795ab2fb2458dca4e1)

Revision: `01491edfa709545786d0fe795ab2fb2458dca4e1`.

Copyright (c) 2025 Vincent Swarte. Licensed under MIT; see [LICENSE-MIT](LICENSE-MIT).

The character lifecycle source provides native task detachment, character-list
removal and allocator-aware destruction. The upstream retirement source remains
in `src/retirement.rs` under this directory; this project's adapted implementation
is selected from the repository-level `src/retirement.rs`.

The byte signatures under `signatures/` are required by native runtime version
checks. They are preserved byte-for-byte from the upstream revision. Their
original directory was relocated and the relative include paths were updated
for this source distribution; the runtime algorithms are unchanged by that
relocation. Original hashes and distributed-file hashes are in `provenance.json`.

The project-specific retirement adaptation adds checked process-memory reads,
identity checks and diagnostics while preserving the native allocator and
destruction sequence.
