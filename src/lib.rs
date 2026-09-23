pub mod session;

#[cfg(all(feature = "native", target_os = "windows"))]
mod native;

// Pinned Summon source; see vendor/summon/PROVENANCE.md. Keep crate-level
// names so the upstream lifecycle and local retirement adaptation interoperate.
#[cfg(all(feature = "native", target_os = "windows"))]
#[allow(dead_code, clippy::collapsible_if)]
#[rustfmt::skip]
#[path = "vendor/summon/src/chr_lifecycle.rs"]
mod chr_lifecycle;
#[cfg(all(feature = "native", target_os = "windows"))]
#[path = "retirement.rs"]
mod retirement;
#[cfg(all(feature = "native", target_os = "windows"))]
fn log_line(message: impl std::fmt::Display) {
    native::log(message);
}
