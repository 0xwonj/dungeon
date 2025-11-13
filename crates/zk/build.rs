//! Build script for zkVM guest program compilation.
//!
//! Conditionally compiles zkVM guest programs based on enabled features:
//! - `risc0` feature: Compiles RISC0 guest and generates ELF/ImageID constants
//! - No features: Generates placeholder constants
//!
//! # Environment Variables
//!
//! - `RISC0_SKIP_BUILD=1`: Skip guest compilation even with risc0 feature (generates placeholders)
//!
//! # Generated Constants
//!
//! Outputs to `OUT_DIR/methods.rs`:
//! - `SINGLE_STATE_TRANSITION_ELF: &[u8]` - Single state transition guest program binary
//! - `SINGLE_STATE_TRANSITION_ID: [u32; 8]` - Single state transition program identifier
//! - `BATCH_STATE_TRANSITION_ELF: &[u8]` - Batch state transition guest program binary
//! - `BATCH_STATE_TRANSITION_ID: [u32; 8]` - Batch state transition program identifier

fn main() {
    use std::env;

    let risc0_enabled = env::var("CARGO_FEATURE_RISC0").is_ok();

    // If risc0 feature is not enabled, just generate placeholders
    if !risc0_enabled {
        generate_placeholder();
        return;
    }

    // risc0 feature is enabled - decide whether to actually build guest
    let should_skip = env::var("RISC0_SKIP_BUILD").is_ok() || is_clippy_or_check();

    if should_skip {
        generate_placeholder();
    } else {
        build_risc0_guest();
    }
}

/// Checks if we're running clippy or cargo check (which don't need guest builds)
fn is_clippy_or_check() -> bool {
    use std::env;

    env::var("CLIPPY_ARGS").is_ok()
        || env::var("RUSTC_WORKSPACE_WRAPPER")
            .map(|v| v.contains("clippy"))
            .unwrap_or(false)
}

/// Generate placeholder constants when guest build is skipped
fn generate_placeholder() {
    use std::env;

    let out_dir = env::var("OUT_DIR").unwrap();
    let methods_path = std::path::Path::new(&out_dir).join("methods.rs");
    std::fs::write(
        methods_path,
        r#"pub const SINGLE_STATE_TRANSITION_ELF: &[u8] = &[];
pub const SINGLE_STATE_TRANSITION_ID: [u32; 8] = [0; 8];
pub const BATCH_STATE_TRANSITION_ELF: &[u8] = &[];
pub const BATCH_STATE_TRANSITION_ID: [u32; 8] = [0; 8];
"#,
    )
    .expect("Failed to write placeholder methods.rs");
}

/// Build RISC0 guest program (only compiled when risc0 feature is enabled)
#[cfg(feature = "risc0")]
fn build_risc0_guest() {
    risc0_build::embed_methods();
}

/// Stub for when risc0 feature is not enabled (should never be called)
#[cfg(not(feature = "risc0"))]
fn build_risc0_guest() {
    unreachable!("build_risc0_guest called without risc0 feature");
}
