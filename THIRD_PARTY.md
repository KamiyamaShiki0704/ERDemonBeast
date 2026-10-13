# Third-party components

## Independent runtime SDK

The runtime uses ShikiFSRS, independently implemented from the owned ER WW 1.17.1
executable and original synthetic tests. Its Rust dependencies and exact versions
are recorded in Cargo.lock. The old runtime binding and vendored lifecycle sources
and signatures have been removed from this candidate.

The serialized creation and retirement adapters reuse the independently rewritten
ER_Summon consumer adapters, maintained by KamiyamaShiki0704. No upstream fsrs
implementation is included in these adapters.

## Interface reference

The original project documented er_enemy_control (Apache-2.0), revision
8056f10c0864e3e10d6c20cc85827676fac36ab4, as a reference for debug control interface
semantics. This historical attribution does not supply the SDK's field layouts or
native function evidence. The project's own form selection, state machine, HKS
bridge and recovery policy are retained.

## Other dependencies and assets

Additional Rust dependencies and exact versions are listed in Cargo.lock and
retain their respective licenses. This repository does not distribute their source
archives or compiled third-party libraries. Its MIT license does not grant rights
to game models, animations, scripts or assets; those assets are not included.
