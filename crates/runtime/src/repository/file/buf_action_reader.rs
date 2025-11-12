//! Buffered sequential reader for completed action log files.
//!
//! This reader is optimized for reading entire action log files from start to finish
//! in a single pass using buffered I/O (BufReader).
//!
//! Use this reader when:
//! - The action log file is complete and won't grow anymore
//! - You need to read from start to finish once (batch processing)
//! - You want simple, straightforward code
//!
//! For tailing growing files, use `MmapActionLogReader` instead.

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use tracing::debug;

use crate::api::Result;
use crate::repository::RepositoryError;
use crate::repository::types::ActionLogEntry;

/// Buffered sequential reader for completed action log files.
///
/// Uses `BufReader` with 8MB buffer for efficient sequential reads.
/// Optimized for reading entire files from start to finish in a single pass.
pub struct BufActionLogReader {
    /// Buffered reader with 8MB buffer for efficient sequential reads
    reader: BufReader<File>,

    /// Session identifier for logging
    session_id: String,

    /// File path being read
    file_path: PathBuf,

    /// Number of bytes read so far
    bytes_read: u64,

    /// Number of entries read so far
    entries_read: u64,
}

impl BufActionLogReader {
    /// Open a completed action log file for sequential reading.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the completed action log file
    /// * `session_id` - Session identifier for logging
    ///
    /// # Errors
    ///
    /// Returns error if the file doesn't exist or cannot be opened.
    pub fn new(file_path: impl AsRef<Path>, session_id: String) -> Result<Self> {
        let file_path = file_path.as_ref().to_path_buf();

        // Open file for reading
        let file = File::open(&file_path).map_err(RepositoryError::Io)?;

        // Get file size for logging
        let file_size = file.metadata().map_err(RepositoryError::Io)?.len();

        // Create buffered reader with 8MB buffer
        let reader = BufReader::with_capacity(8 * 1024 * 1024, file);

        debug!(
            "Opened completed action log for batch reading: {} ({} bytes)",
            file_path.display(),
            file_size
        );

        Ok(Self {
            reader,
            session_id,
            file_path,
            bytes_read: 0,
            entries_read: 0,
        })
    }

    /// Read the next action log entry.
    ///
    /// Returns `None` when reaching the end of the file.
    ///
    /// # Errors
    ///
    /// - `Serialization` - Failed to deserialize entry
    /// - `Io` - I/O error during read
    ///
    /// # Performance
    ///
    /// Uses buffered I/O with 8MB buffer for efficient sequential reads.
    /// The OS page cache is still utilized for optimal performance.
    pub fn read_next(&mut self) -> Result<Option<ActionLogEntry>> {
        // Read length prefix (4 bytes)
        let mut len_bytes = [0u8; 4];
        match self.reader.read_exact(&mut len_bytes) {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                // Reached end of file
                return Ok(None);
            }
            Err(e) => return Err(RepositoryError::Io(e).into()),
        }

        let len = u32::from_le_bytes(len_bytes) as usize;
        self.bytes_read += 4;

        // Read data
        let mut data = vec![0u8; len];
        self.reader
            .read_exact(&mut data)
            .map_err(RepositoryError::Io)?;
        self.bytes_read += len as u64;

        // Deserialize entry
        let entry: ActionLogEntry = bincode::deserialize(&data)
            .map_err(|e| RepositoryError::Serialization(e.to_string()))?;

        self.entries_read += 1;

        Ok(Some(entry))
    }

    /// Read all remaining entries into a vector.
    ///
    /// Convenient method for reading entire file contents at once.
    /// Useful for batch proof generation.
    pub fn read_all(&mut self) -> Result<Vec<ActionLogEntry>> {
        let mut entries = Vec::new();
        while let Some(entry) = self.read_next()? {
            entries.push(entry);
        }
        Ok(entries)
    }

    /// Get the number of bytes read so far.
    pub fn bytes_read(&self) -> u64 {
        self.bytes_read
    }

    /// Get the number of entries read so far.
    pub fn entries_read(&self) -> u64 {
        self.entries_read
    }

    /// Get the session ID.
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Get the file path.
    pub fn file_path(&self) -> &Path {
        &self.file_path
    }
}

// Note: Integration tests for BufActionLogReader will be done at the runtime level
// where real GameState instances are available. Unit tests are skipped here because
// creating valid ActionLogEntry instances requires complex GameState setup.
