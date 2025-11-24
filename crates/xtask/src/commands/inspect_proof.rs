//! Inspect and debug ZK proof files
//!
//! This command loads proof files from the persistence layer and displays
//! detailed information about their structure and contents.

use anyhow::{Context, Result, anyhow};
use clap::Parser;

#[derive(Debug, Parser)]
pub struct InspectProof {
    /// Nonce of the proof to inspect (e.g., "0" for proof_0.bin)
    nonce: u64,

    /// Session ID (defaults to latest session)
    #[arg(short, long)]
    session: Option<String>,

    /// Show raw proof bytes (first N bytes)
    #[arg(long, default_value = "64")]
    show_bytes: usize,

    /// Validate proof structure and journal
    #[arg(long)]
    validate: bool,
}

impl InspectProof {
    pub fn run(&self) -> Result<()> {
        // Determine session directory
        let session_id = if let Some(ref sid) = self.session {
            sid.clone()
        } else {
            // Find latest session
            crate::utils::find_latest_session()?
        };

        // Construct proof directory path
        let data_dir = crate::utils::data_dir()?;
        let proof_dir = data_dir.join(&session_id).join("proofs");

        if !proof_dir.exists() {
            return Err(anyhow!(
                "Proofs directory not found: {}\n\
                 Session: {}",
                proof_dir.display(),
                session_id
            ));
        }

        // Find proof file that contains this nonce
        // Files are named: proof_<start_nonce>_<end_nonce>.bin
        let mut proof_path = None;
        for entry in std::fs::read_dir(&proof_dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                if let Some(stripped) = filename
                    .strip_prefix("proof_")
                    .and_then(|s| s.strip_suffix(".bin"))
                {
                    if let Some((start_str, end_str)) = stripped.split_once('_') {
                        if let (Ok(start), Ok(end)) =
                            (start_str.parse::<u64>(), end_str.parse::<u64>())
                        {
                            if self.nonce >= start && self.nonce <= end {
                                proof_path = Some(path);
                                break;
                            }
                        }
                    }
                }
            }
        }

        let proof_path = proof_path.ok_or_else(|| {
            anyhow!(
                "No proof file found containing nonce {}\n\
                 Proof directory: {}\n\
                 Session: {}",
                self.nonce,
                proof_dir.display(),
                session_id
            )
        })?;

        println!("📁 Proof File: {}", proof_path.display());
        println!("📊 Session: {}", session_id);
        println!("🔢 Nonce: {}", self.nonce);
        println!();

        // Read proof file
        let proof_bytes = std::fs::read(&proof_path)
            .with_context(|| format!("Failed to read proof file: {}", proof_path.display()))?;

        println!("📦 File Size: {} bytes", proof_bytes.len());
        println!();

        // Try to deserialize as ProofData
        match bincode::deserialize::<zk::ProofData>(&proof_bytes) {
            Ok(proof_data) => {
                self.display_proof_data(&proof_data)?;

                if self.validate {
                    println!();
                    self.validate_proof(&proof_data)?;
                }
            }
            Err(e) => {
                println!("❌ Failed to deserialize proof: {}", e);
                println!();
                println!("Raw bytes (first {} bytes):", self.show_bytes);
                self.display_hex_dump(&proof_bytes, self.show_bytes);
            }
        }

        Ok(())
    }

    fn display_proof_data(&self, proof: &zk::ProofData) -> Result<()> {
        println!("✅ Successfully deserialized ProofData");
        println!();

        println!("🔐 Proof Backend: {:?}", proof.backend);
        println!("📏 Proof Bytes: {} bytes", proof.bytes.len());
        println!("📜 Journal (Public Values): {} bytes", proof.journal.len());
        println!("🔑 Journal Digest: {}", hex::encode(proof.journal_digest));
        println!();

        // Display journal structure
        if proof.journal.len() == 168 {
            println!("📖 Journal Structure (168 bytes):");
            println!(
                "  ├─ oracle_root       (bytes 0..32):   {}",
                hex::encode(&proof.journal[0..32])
            );
            println!(
                "  ├─ seed_commitment   (bytes 32..64):  {}",
                hex::encode(&proof.journal[32..64])
            );
            println!(
                "  ├─ prev_state_root   (bytes 64..96):  {}",
                hex::encode(&proof.journal[64..96])
            );
            println!(
                "  ├─ actions_root      (bytes 96..128): {}",
                hex::encode(&proof.journal[96..128])
            );
            println!(
                "  ├─ new_state_root    (bytes 128..160):{}",
                hex::encode(&proof.journal[128..160])
            );
            println!(
                "  └─ new_nonce         (bytes 160..168): {} (0x{})",
                u64::from_le_bytes(proof.journal[160..168].try_into().unwrap()),
                hex::encode(&proof.journal[160..168])
            );
        } else if proof.journal.is_empty() {
            println!("⚠️  WARNING: Journal is EMPTY (0 bytes)");
            println!("   This indicates a problem with proof generation!");
            println!("   Expected: 168 bytes containing public values");
        } else {
            println!(
                "⚠️  WARNING: Journal has unexpected size: {} bytes",
                proof.journal.len()
            );
            println!("   Expected: 168 bytes");
            println!("   Raw journal bytes:");
            self.display_hex_dump(&proof.journal, proof.journal.len().min(256));
        }

        println!();

        // Show proof bytes preview
        if self.show_bytes > 0 {
            println!(
                "🔍 Proof Bytes (first {} bytes):",
                self.show_bytes.min(proof.bytes.len())
            );
            self.display_hex_dump(&proof.bytes, self.show_bytes);
            println!();
        }

        Ok(())
    }

    fn validate_proof(&self, proof: &zk::ProofData) -> Result<()> {
        println!("🔬 Validating Proof Structure...");
        println!();

        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Check journal length
        if proof.journal.is_empty() {
            errors.push("Journal is empty (0 bytes)".to_string());
        } else if proof.journal.len() != 168 {
            errors.push(format!(
                "Journal has incorrect size: {} bytes (expected 168)",
                proof.journal.len()
            ));
        }

        // Check journal digest
        let computed_digest = zk::compute_journal_digest(&proof.journal);
        if computed_digest != proof.journal_digest {
            errors.push(format!(
                "Journal digest mismatch:\n  Stored:   {}\n  Computed: {}",
                hex::encode(proof.journal_digest),
                hex::encode(computed_digest)
            ));
        }

        // Check proof bytes
        if proof.bytes.is_empty() {
            errors.push("Proof bytes are empty".to_string());
        } else if proof.bytes.len() < 100 {
            warnings.push(format!(
                "Proof bytes seem small: {} bytes",
                proof.bytes.len()
            ));
        }

        // Check backend-specific constraints based on debug format
        let backend_str = format!("{:?}", proof.backend);
        if backend_str.contains("Sp1") {
            // SP1 Groth16 proofs should be relatively small (~300-500 bytes for proof itself)
            // But the full SP1ProofWithPublicValues can be larger
            if proof.bytes.len() < 200 {
                warnings.push(format!(
                    "SP1 proof seems small: {} bytes",
                    proof.bytes.len()
                ));
            }
        } else if backend_str.contains("Risc0") {
            // RISC0 proofs are typically larger
            if proof.bytes.len() < 1000 {
                warnings.push(format!(
                    "RISC0 proof seems small: {} bytes",
                    proof.bytes.len()
                ));
            }
        }
        // Stub proofs are dummy, no size requirements

        // Display results
        if errors.is_empty() && warnings.is_empty() {
            println!("✅ Validation PASSED - No issues found");
        } else {
            if !errors.is_empty() {
                println!("❌ Validation FAILED - {} error(s):", errors.len());
                for (i, error) in errors.iter().enumerate() {
                    println!("   {}. {}", i + 1, error);
                }
            }

            if !warnings.is_empty() {
                println!();
                println!("⚠️  Warnings ({}):", warnings.len());
                for (i, warning) in warnings.iter().enumerate() {
                    println!("   {}. {}", i + 1, warning);
                }
            }

            if !errors.is_empty() {
                return Err(anyhow!("Proof validation failed"));
            }
        }

        Ok(())
    }

    fn display_hex_dump(&self, data: &[u8], max_bytes: usize) {
        let bytes_to_show = data.len().min(max_bytes);

        for (i, chunk) in data[..bytes_to_show].chunks(16).enumerate() {
            print!("  {:04x}:  ", i * 16);

            // Hex bytes
            for (j, byte) in chunk.iter().enumerate() {
                if j == 8 {
                    print!(" ");
                }
                print!("{:02x} ", byte);
            }

            // Padding
            if chunk.len() < 16 {
                for j in chunk.len()..16 {
                    if j == 8 {
                        print!(" ");
                    }
                    print!("   ");
                }
            }

            // ASCII representation
            print!(" |");
            for byte in chunk {
                if byte.is_ascii_graphic() || *byte == b' ' {
                    print!("{}", *byte as char);
                } else {
                    print!(".");
                }
            }
            println!("|");
        }

        if bytes_to_show < data.len() {
            println!("  ... ({} more bytes)", data.len() - bytes_to_show);
        }
    }
}
