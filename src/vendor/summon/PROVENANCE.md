# Summon source reuse

Source: local `D:/_Ugit/FSTool/_Mod/_Project/Summon`, upstream `https://github.com/KamiyamaShiki0704/ER_Summon`, revision `01491edfa709545786d0fe795ab2fb2458dca4e1` (clean checkout when copied).

Copied without edits: `src/chr_lifecycle.rs`, `src/retirement.rs`, the three binary fixtures referenced by those modules, and `LICENSE-MIT`. The sources retain their crate-level module paths through this project's `src/lib.rs`. Rustfmt traversal is disabled for these two pinned files; their existing collapsible-if style is allowed locally. Our adapter is in `src/native.rs`.

Reused runtime path: checked native task unregistration and ChrSet detachment, original allocator ownership, two quiet FrameEnd observations, destructor without CRT deletion, original allocator deallocation. No calling interface is exported by the sibling DLL, so this is source reuse rather than a call into an installed Summon.dll.

Spawn construction follows the same pinned Summon `spawn_debug_character` layout (init data 0x100, rotation +0x10, is_player +0xC8); this project's ownership binding is stricter and does not consume another plugin's last-created pointer.

The original Summon project is read-only. Its runtime success is reference evidence, not an ER Demon Beast acceptance result. Source hashes are retained in `provenance.json`; all copied fixtures are verified against the exact local target EXE before packaging.
