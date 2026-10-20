# Summon source reuse

Source: local `D:/_Ugit/FSTool/_Mod/_Project/Summon`, upstream `https://github.com/KamiyamaShiki0704/ER_Summon`, revision `01491edfa709545786d0fe795ab2fb2458dca4e1` (clean checkout when copied).

Copied without edits: `src/chr_lifecycle.rs`, `src/retirement.rs`, the three binary fixtures referenced by those modules, and `LICENSE-MIT`. The sources retain their crate-level module paths through this project's `src/lib.rs`. Rustfmt traversal is disabled for these two pinned files; their existing collapsible-if style is allowed locally. Our adapter is in `src/native.rs`.

Reused runtime path: checked native task unregistration and ChrSet detachment, original allocator ownership, two quiet FrameEnd observations, destructor without CRT deletion, original allocator deallocation. No calling interface is exported by the sibling DLL, so this is source reuse rather than a call into an installed Summon.dll.

Spawn construction follows the same pinned Summon `spawn_debug_character` layout (init data 0x100, rotation +0x10, is_player +0xC8); this project's ownership binding is stricter and does not consume another plugin's last-created pointer.

The original Summon project is read-only. Its runtime success is reference evidence, not an ER Demon Beast acceptance result. Source hashes are retained in `provenance.json`; all copied fixtures are verified against the exact local target EXE before packaging.

## Local adaptation in 0.2.1

All copied source/fixture/license files under this vendor directory remain the original bytes. `src/lib.rs` now selects `src/retirement.rs`, derived from the pinned retirement source, with checked full-span process-memory copies, stable vector headers and rate-limited failure diagnostics. The two-quiet-observation gate and allocator/destructor protocol remain unchanged. See `docs/feat-er-demon-beast/research/retirement-stall-0.2.1.md`.

## Local 1.17.2 adapter (2026-10-21)

The pinned files here remain unchanged. The active local `src/chr_lifecycle.rs` adapter adds separate WW272 byte fixtures and uses fsrs versioned WorldChrMan tail access. `src/lib.rs` selects that adapter; local retirement keeps its checked reader and deferred ownership protocol. Current hashes and acceptance boundary are in the shared 1.17.2 build record.
