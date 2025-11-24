//! Extract SP1 Groth16 verifying key from a proof file.
//!
//! This command generates a dummy SP1 Groth16 proof and extracts the verifying key,
//! saving it to a file for use in Sui contract deployment.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use ark_serialize::CanonicalSerialize;
use clap::Parser;

/// Extract SP1 Groth16 VK from a generated proof
#[derive(Debug, Parser)]
pub struct ExtractVk {
    /// Output path for the VK bytes (arkworks format)
    #[arg(short, long, default_value = "vk_5.2_ark.bin")]
    pub output: String,

    /// Input proof file path
    #[arg(short, long, default_value = "sp1_groth16_proof.bin")]
    pub proof: String,
}

impl ExtractVk {
    /// Execute the extract-vk command.
    pub fn execute(&self) -> Result<()> {
        println!("🔑 Extracting SP1 Groth16 VK from proof...");
        println!();

        // Check if we have a proof file to extract from
        let proof_path = PathBuf::from(&self.proof);

        if proof_path.exists() {
            println!("📂 Found proof file: {}", proof_path.display());
            self.extract_from_proof_file(&proof_path)?;
        } else {
            println!("⚠️  No proof file found at {}", proof_path.display());
            println!();
            println!("To generate a Groth16 proof:");
            println!("  1. Set: export SP1_PROOF_MODE=groth16");
            println!("  2. Set: export SP1_PROVER=network  # (or cpu)");
            println!(
                "  3. Run: cargo run -p dungeon-client --no-default-features --features \"cli,sui,sp1\""
            );
            println!("  4. Play game to generate proof");
            println!(
                "  5. Copy proof: cp ~/.cache/dungeon/sessions/*/proofs/*.bin sp1_groth16_proof.bin"
            );
            println!("  6. Run: cargo xtask extract-vk");
            return Ok(());
        }

        Ok(())
    }

    /// Extract VK from an existing proof file.
    fn extract_from_proof_file(&self, proof_path: &PathBuf) -> Result<()> {
        // Read proof file
        let proof_bytes = fs::read(proof_path).context("Failed to read proof file")?;

        println!("   Proof size: {} bytes", proof_bytes.len());

        // Deserialize as zk::ProofData (our wrapper format)
        let proof_data: zk::ProofData =
            bincode::deserialize(&proof_bytes).context("Failed to deserialize ProofData")?;

        // Verify this is an SP1 proof
        if proof_data.backend != zk::ProofBackend::Sp1 {
            anyhow::bail!(
                "Proof is not from SP1 backend (found: {:?})",
                proof_data.backend
            );
        }

        println!("   Backend: SP1");
        println!("   Journal size: {} bytes", proof_data.journal.len());

        // Deserialize inner SP1 proof
        let sp1_proof: sp1_sdk::SP1ProofWithPublicValues = bincode::deserialize(&proof_data.bytes)
            .context("Failed to deserialize inner SP1ProofWithPublicValues")?;

        println!("   SP1 proof type: {:?}", sp1_proof.proof);

        // Extract VK using sp1-sui converter (this gives us both gnark and arkworks VK)
        println!("   Converting proof to arkworks format...");
        let (vk_ark, _public_inputs, _proof_points) = sp1_sui::convert_sp1_gnark_to_ark(sp1_proof);

        println!("   ✓ VK extracted successfully!");
        println!();

        // The gnark VK is embedded in the original proof, but we need to extract it differently
        // For now, we'll serialize the arkworks VK

        // Serialize arkworks VK
        let mut ark_bytes = Vec::new();
        vk_ark
            .serialize_compressed(&mut ark_bytes)
            .context("Failed to serialize arkworks VK")?;

        // Save arkworks VK
        fs::write(&self.output, &ark_bytes).context("Failed to write arkworks VK file")?;

        println!("✅ Arkworks VK saved:");
        println!("   File: {}", self.output);
        println!("   Size: {} bytes", ark_bytes.len());
        println!();

        // Print Rust code to paste into sp1-sui
        println!("📝 To use this VK, update the constant in your code:");
        println!();
        println!("```rust");
        println!("// In crates/xtask/src/commands/sui/setup.rs or similar");
        println!("const GROTH16_VK_5_2_BYTES: &[u8] = &[");
        for (i, chunk) in ark_bytes.chunks(16).enumerate() {
            print!("    ");
            for byte in chunk {
                print!("0x{:02x}, ", byte);
            }
            if i < ark_bytes.chunks(16).count() - 1 {
                println!();
            }
        }
        println!();
        println!("];");
        println!("```");
        println!();

        println!("🎉 VK extraction complete!");
        println!("   Use this VK in your Sui deployment setup.");

        Ok(())
    }
}
