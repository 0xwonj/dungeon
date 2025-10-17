//! Memory-mapped action log reader for high-performance sequential access.
//!
//! This module provides zero-copy reading of action log files using memory mapping.
//! The OS handles page caching and read-ahead automatically, providing optimal
//! performance for sequential access patterns (ProverWorker).

use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use memmap2::Mmap;
use tracing::debug;

use crate::api::Result;
use crate::repository::traits::ActionLogReader;
use crate::repository::types::ActionLogEntry;
use crate::repository::RepositoryError;

/// Memory-mapped action log reader optimized for sequential access.
///
/// # Design
///
/// - Uses memory mapping for zero-copy reads
/// - OS handles page caching and read-ahead automatically
/// - Thread-safe with atomic operations for offset tracking
/// - Automatic remapping when file grows
///
/// # Performance
///
/// - Zero syscalls after initial mmap (except remapping)
/// - Zero memory copies (read directly from page cache)
/// - OS-level read-ahead for sequential patterns
/// - Minimal memory usage (OS manages page cache)
///
/// # Thread Safety
///
/// Multiple threads can safely read concurrently. The reader uses:
/// - `AtomicU64` for current offset (lock-free reads)
/// - `RwLock<Mmap>` for remapping (rare writes, frequent reads)
pub struct MmapActionLogReader {
    /// Memory-mapped file (shared read-only access)
    mmap: Arc<RwLock<Mmap>>,

    /// Current read offset (bytes)
    current_offset: AtomicU64,

    /// File size at last check (bytes)
    file_size: AtomicU64,

    /// Path to the action log file
    file_path: PathBuf,

    /// Session identifier
    session_id: String,
}

impl MmapActionLogReader {
    /// Create a new memory-mapped reader for an action log file.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the action log file
    /// * `session_id` - Session identifier for logging
    /// * `start_offset` - Starting byte offset (0 for beginning, or checkpoint offset)
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - File doesn't exist
    /// - Cannot memory map the file
    /// - Invalid start offset
    pub fn new(
        file_path: impl AsRef<Path>,
        session_id: String,
        start_offset: u64,
    ) -> Result<Self> {
        let file_path = file_path.as_ref().to_path_buf();

        // Open file for reading
        let file = File::open(&file_path).map_err(RepositoryError::Io)?;

        // Get file size
        let metadata = file.metadata().map_err(RepositoryError::Io)?;
        let file_size = metadata.len();

        // Validate start offset
        if start_offset > file_size {
            return Err(RepositoryError::InvalidOffset {
                offset: start_offset,
                file_size,
            }
            .into());
        }

        // Memory map the file
        let mmap = unsafe { Mmap::map(&file).map_err(RepositoryError::Io)? };

        debug!(
            "Memory-mapped action log: {} ({} bytes, starting at offset {})",
            file_path.display(),
            file_size,
            start_offset
        );

        Ok(Self {
            mmap: Arc::new(RwLock::new(mmap)),
            current_offset: AtomicU64::new(start_offset),
            file_size: AtomicU64::new(file_size),
            file_path,
            session_id,
        })
    }

    /// Read the next action log entry from the current offset.
    ///
    /// This is a zero-copy operation that reads directly from the memory-mapped region.
    /// The OS handles page faults and read-ahead automatically.
    ///
    /// # Returns
    ///
    /// - `Some(entry)` - Next entry read successfully
    /// - `None` - Reached end of file (caught up with writer)
    ///
    /// # Thread Safety
    ///
    /// Safe to call from multiple threads, but each thread should track its own offset
    /// or use external synchronization.
    pub fn read_next(&self) -> Result<Option<ActionLogEntry>> {
        let offset = self.current_offset.load(Ordering::Acquire);
        let size = self.file_size.load(Ordering::Acquire);

        // Check if we've reached the end
        if offset >= size {
            return Ok(None);
        }

        // Get read lock on mmap (allows concurrent readers)
        let mmap = self
            .mmap
            .read()
            .map_err(|_| RepositoryError::LockPoisoned)?;

        // Read from memory (zero-copy)
        let data = &mmap[offset as usize..];

        // Parse entry: [u32 length][bincode data]
        if data.len() < 4 {
            // Not enough data for length prefix
            return Ok(None);
        }

        let len = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;

        // Check if we have enough data for the full entry
        if data.len() < 4 + len {
            return Ok(None);
        }

        // Deserialize entry
        let entry: ActionLogEntry = bincode::deserialize(&data[4..4 + len])
            .map_err(|e| RepositoryError::Serialization(e.to_string()))?;

        // Update offset atomically
        let new_offset = offset + 4 + len as u64;
        self.current_offset.store(new_offset, Ordering::Release);

        Ok(Some(entry))
    }

    /// Peek at the next entry without advancing the offset.
    ///
    /// Useful for checking if more data is available without consuming it.
    pub fn peek_next(&self) -> Result<Option<ActionLogEntry>> {
        let offset = self.current_offset.load(Ordering::Acquire);
        let size = self.file_size.load(Ordering::Acquire);

        if offset >= size {
            return Ok(None);
        }

        let mmap = self
            .mmap
            .read()
            .map_err(|_| RepositoryError::LockPoisoned)?;

        let data = &mmap[offset as usize..];

        if data.len() < 4 {
            return Ok(None);
        }

        let len = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;

        if data.len() < 4 + len {
            return Ok(None);
        }

        let entry: ActionLogEntry = bincode::deserialize(&data[4..4 + len])
            .map_err(|e| RepositoryError::Serialization(e.to_string()))?;

        Ok(Some(entry))
    }

    /// Refresh file size and remap if the file has grown.
    ///
    /// This should be called periodically when waiting for new data.
    /// The OS will automatically pick up new pages, but we need to update
    /// our size tracking and potentially remap if the file grew significantly.
    ///
    /// # Performance
    ///
    /// - Fast path: Just checks file metadata (single syscall)
    /// - Slow path: Remaps file if grown (rare, amortized cost)
    pub fn refresh(&self) -> Result<bool> {
        // Check file size
        let metadata = std::fs::metadata(&self.file_path).map_err(RepositoryError::Io)?;
        let new_size = metadata.len();
        let old_size = self.file_size.load(Ordering::Acquire);

        if new_size <= old_size {
            return Ok(false); // No growth
        }

        // File has grown - remap
        let file = File::open(&self.file_path).map_err(RepositoryError::Io)?;
        let new_mmap = unsafe { Mmap::map(&file).map_err(RepositoryError::Io)? };

        // Update mmap (requires write lock)
        let mut mmap = self
            .mmap
            .write()
            .map_err(|_| RepositoryError::LockPoisoned)?;
        *mmap = new_mmap;

        // Update size
        self.file_size.store(new_size, Ordering::Release);

        debug!(
            "Remapped action log: {} -> {} bytes",
            old_size, new_size
        );

        Ok(true)
    }

    /// Get the current read offset.
    pub fn current_offset(&self) -> u64 {
        self.current_offset.load(Ordering::Acquire)
    }

    /// Get the current file size.
    pub fn file_size(&self) -> u64 {
        self.file_size.load(Ordering::Acquire)
    }

    /// Get the session ID.
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Get the file path.
    pub fn file_path(&self) -> &Path {
        &self.file_path
    }

    /// Check if there's more data available to read.
    pub fn has_more(&self) -> bool {
        self.current_offset.load(Ordering::Acquire) < self.file_size.load(Ordering::Acquire)
    }

    /// Get the number of bytes remaining to read.
    pub fn bytes_remaining(&self) -> u64 {
        let offset = self.current_offset.load(Ordering::Acquire);
        let size = self.file_size.load(Ordering::Acquire);
        size.saturating_sub(offset)
    }
}

// Thread-safe: Multiple threads can read concurrently
unsafe impl Send for MmapActionLogReader {}
unsafe impl Sync for MmapActionLogReader {}

// Implement ActionLogReader trait
impl ActionLogReader for MmapActionLogReader {
    fn read_next(&self) -> Result<Option<ActionLogEntry>> {
        self.read_next()
    }

    fn refresh(&self) -> Result<bool> {
        self.refresh()
    }

    fn current_offset(&self) -> u64 {
        self.current_offset()
    }

    fn session_id(&self) -> &str {
        self.session_id()
    }

    fn has_more(&self) -> bool {
        self.has_more()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::file::FileRepository;
    use game_core::{Action, CharacterActionKind, EntityId, GameState};
    use tempfile::TempDir;

    fn create_test_entry(nonce: u64) -> ActionLogEntry {
        let before_state = GameState::default();
        let after_state = GameState::default();
        let action = Action::character(EntityId::PLAYER, CharacterActionKind::Wait);

        ActionLogEntry {
            nonce,
            clock: nonce,
            action: action.clone(),
            before_state: Box::new(before_state.clone()),
            after_state: Box::new(after_state.clone()),
            delta: None, // Delta not needed for tests
        }
    }

    #[test]
    fn test_mmap_reader_basic() {
        let temp_dir = TempDir::new().unwrap();
        let filename = "test_actions.log";

        // Write some entries
        {
            let mut writer =
                FileRepository::<ActionLogEntry>::create(temp_dir.path(), filename).unwrap();
            for i in 0..10 {
                writer.append(&create_test_entry(i)).unwrap();
            }
            writer.flush().unwrap();
        }

        // Read with mmap
        let reader = MmapActionLogReader::new(
            temp_dir.path().join(filename),
            "test".to_string(),
            0,
        )
        .unwrap();

        // Read all entries
        let mut count = 0;
        while let Some(entry) = reader.read_next().unwrap() {
            assert_eq!(entry.nonce, count);
            count += 1;
        }

        assert_eq!(count, 10);
    }

    #[test]
    fn test_mmap_reader_refresh() {
        let temp_dir = TempDir::new().unwrap();
        let filename = "test_actions.log";

        // Write initial entries
        let mut writer =
            FileRepository::<ActionLogEntry>::create(temp_dir.path(), filename).unwrap();
        for i in 0..5 {
            writer.append(&create_test_entry(i)).unwrap();
        }
        writer.flush().unwrap();

        // Create reader
        let reader = MmapActionLogReader::new(
            temp_dir.path().join(filename),
            "test".to_string(),
            0,
        )
        .unwrap();

        // Read first batch
        let mut count = 0;
        while let Some(entry) = reader.read_next().unwrap() {
            assert_eq!(entry.nonce, count);
            count += 1;
        }
        assert_eq!(count, 5);

        // Write more entries
        for i in 5..10 {
            writer.append(&create_test_entry(i)).unwrap();
        }
        writer.flush().unwrap();

        // Refresh and read new entries
        let grew = reader.refresh().unwrap();
        assert!(grew);

        while let Some(entry) = reader.read_next().unwrap() {
            assert_eq!(entry.nonce, count);
            count += 1;
        }
        assert_eq!(count, 10);
    }

    #[test]
    fn test_mmap_reader_peek() {
        let temp_dir = TempDir::new().unwrap();
        let filename = "test_actions.log";

        // Write entries
        let mut writer =
            FileRepository::<ActionLogEntry>::create(temp_dir.path(), filename).unwrap();
        writer.append(&create_test_entry(0)).unwrap();
        writer.flush().unwrap();

        // Create reader
        let reader = MmapActionLogReader::new(
            temp_dir.path().join(filename),
            "test".to_string(),
            0,
        )
        .unwrap();

        // Peek should not advance offset
        let peeked = reader.peek_next().unwrap().unwrap();
        assert_eq!(peeked.nonce, 0);
        assert_eq!(reader.current_offset(), 0);

        // Peek again - same result
        let peeked = reader.peek_next().unwrap().unwrap();
        assert_eq!(peeked.nonce, 0);

        // Now read - should advance offset
        let entry = reader.read_next().unwrap().unwrap();
        assert_eq!(entry.nonce, 0);
        assert!(reader.current_offset() > 0);

        // Peek at end - should be None
        assert!(reader.peek_next().unwrap().is_none());
    }
}
