//! Build script for zkVM guest program compilation.
//!
//! Conditionally compiles zkVM guest programs based on enabled features:
//! - `risc0` feature: Compiles RISC0 guest and generates ELF/ImageID constants
//! - `sp1` feature: Compiles SP1 guest and generates ELF constant
//! - No features: Generates placeholder constants
//!
//! # Environment Variables
//!
//! - `RISC0_SKIP_BUILD=1`: Skip RISC0 guest compilation (generates placeholders)
//!
//! # Generated Constants
//!
//! Outputs to `OUT_DIR/methods.rs`:
//!
//! **RISC0:**
//! - `STATE_TRANSITION_ELF: &[u8]` - State transition guest program binary
//! - `STATE_TRANSITION_ID: [u32; 8]` - State transition program identifier
//!
//! **SP1:**
//! - `STATE_TRANSITION_ELF: &[u8]` - State transition guest program binary

fn main() {
    use std::env;

    let risc0_enabled = env::var("CARGO_FEATURE_RISC0").is_ok();
    let sp1_enabled = env::var("CARGO_FEATURE_SP1").is_ok();

    // Exactly one backend should be enabled (enforced by lib.rs compile_error!)
    // But double-check here for safety
    if risc0_enabled && sp1_enabled {
        panic!("Both risc0 and sp1 features enabled - this should be prevented by lib.rs");
    }

    if risc0_enabled {
        // RISC0 backend
        let should_skip = env::var("RISC0_SKIP_BUILD").is_ok() || is_clippy_or_check();
        if should_skip {
            generate_risc0_placeholder();
        } else {
            build_risc0_guest();
        }
    } else if sp1_enabled {
        // SP1 backend
        let should_skip = is_clippy_or_check();
        if should_skip {
            generate_sp1_placeholder();
        } else {
            build_sp1_guest();
        }
    } else {
        // No backend enabled (stub or arkworks)
        generate_risc0_placeholder(); // Compatible placeholder
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

/// Generate RISC0 placeholder constants when guest build is skipped
fn generate_risc0_placeholder() {
    use std::env;

    let out_dir = env::var("OUT_DIR").unwrap();
    let methods_path = std::path::Path::new(&out_dir).join("methods.rs");
    std::fs::write(
        methods_path,
        r#"pub const STATE_TRANSITION_ELF: &[u8] = &[];
pub const STATE_TRANSITION_ID: [u32; 8] = [0; 8];
"#,
    )
    .expect("Failed to write RISC0 placeholder methods.rs");
}

/// Generate SP1 placeholder constants when guest build is skipped
fn generate_sp1_placeholder() {
    use std::env;

    let out_dir = env::var("OUT_DIR").unwrap();
    let methods_path = std::path::Path::new(&out_dir).join("methods.rs");
    std::fs::write(
        methods_path,
        r#"pub const STATE_TRANSITION_ELF: &[u8] = &[];
"#,
    )
    .expect("Failed to write SP1 placeholder methods.rs");
}

/// Build RISC0 guest program (only compiled when risc0 feature is enabled)
#[cfg(feature = "risc0")]
fn build_risc0_guest() {
    println!("cargo:rerun-if-changed=methods/risc0/");

    // Debug: print current directory and methods path
    if let Ok(current_dir) = std::env::current_dir() {
        eprintln!("Current directory: {:?}", current_dir);
        eprintln!(
            "RISC0 methods path: {:?}",
            current_dir.join("methods/risc0")
        );
        eprintln!(
            "RISC0 methods exists: {}",
            current_dir.join("methods/risc0").exists()
        );
    }

    risc0_build::embed_methods();
}

/// Stub for when risc0 feature is not enabled (should never be called)
#[cfg(not(feature = "risc0"))]
fn build_risc0_guest() {
    unreachable!("build_risc0_guest called without risc0 feature");
}

/// Build SP1 guest program (only compiled when sp1 feature is enabled)
#[cfg(feature = "sp1")]
fn build_sp1_guest() {
    use sp1_build::build_program;
    use std::env;
    use std::fs;
    use std::path::Path;

    println!("cargo:rerun-if-changed=methods/sp1/");

    // Build SP1 guest program - this generates the ELF binary
    build_program("methods/sp1/state-transition");

    // SP1's build_program doesn't automatically generate methods.rs like RISC0's embed_methods
    // We need to manually generate it
    let out_dir = env::var("OUT_DIR").unwrap();
    let methods_path = Path::new(&out_dir).join("methods.rs");

    // The ELF binary is created by build_program in methods/sp1/target/elf-compilation/
    // Relative path from OUT_DIR (target/debug/build/zk-*/out) to the ELF file
    let elf_path = "../../../../../crates/zk/methods/sp1/target/elf-compilation/riscv32im-succinct-zkvm-elf/release/state-transition";

    // Generate methods.rs that includes the ELF as a byte slice
    let methods_content = format!(
        r#"pub const STATE_TRANSITION_ELF: &[u8] = include_bytes!("{}");"#,
        elf_path
    );

    fs::write(methods_path, methods_content).expect("Failed to write SP1 methods.rs");
}

/// Stub for when sp1 feature is not enabled (should never be called)
#[cfg(not(feature = "sp1"))]
fn build_sp1_guest() {
    unreachable!("build_sp1_guest called without sp1 feature");
}
