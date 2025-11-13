//! Append-only log file repository.
//!
//! This module provides a generic `FileRepository<T>` that can store any
//! serializable type in an append-only log format. It is used for both
//! the event log (Event) and action log (ActionLogEntry).

use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use serde::{Serialize, de::DeserializeOwned};

use crate::events::Event;
use crate::repository::traits::{ActionLogWriter, EventRepository};
use crate::repository::types::ActionLogEntry;
use crate::repository::{RepositoryError, Result};

// ============================================================================
// Generic FileRepository
// ============================================================================

/// Generic file-based repository for append-only logs.
///
/// Stores items of type `T` in a file using the format:
/// ```text
/// [u32 length][bincode serialized T]
/// [u32 length][bincode serialized T]
/// ...
/// ```
///
/// # Type Parameters
///
/// - `T`: The type to store. Must implement `Serialize + DeserializeOwned`.
///
/// # Fields
///
/// - `session_id`: Filename identifier (e.g., "events.log", "actions.log")
/// - `path`: Full file path for this log
/// - `writer`: Buffered writer with 8MB buffer for performance
/// - `current_offset`: Current write position in bytes
pub struct FileRepository<T> {
    /// Session identifier (filename without path)
    session_id: String,
    /// Full path to the log file
    path: PathBuf,
    /// Buffered writer (8MB buffer)
    writer: BufWriter<File>,
    /// Current byte offset for next write
    current_offset: u64,
    /// Phantom type marker
    _phantom: PhantomData<T>,
}

impl<T> FileRepository<T>
where
    T: Serialize + DeserializeOwned,
{
    /// Create a new file repository.
    ///
    /// # Errors
    ///
    /// Returns error if the file already exists (prevents accidental overwrites).
    pub fn create(base_dir: impl AsRef<Path>, filename: impl AsRef<str>) -> Result<Self> {
        let base_dir = base_dir.as_ref();
        let filename = filename.as_ref();

        std::fs::create_dir_all(base_dir).map_err(RepositoryError::Io)?;

        let path = base_dir.join(filename);

        // Prevent overwriting existing logs
        if path.exists() {
            return Err(RepositoryError::LogAlreadyExists(
                path.display().to_string(),
            ));
        }

        let file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .map_err(RepositoryError::Io)?;

        let writer = BufWriter::with_capacity(8 * 1024 * 1024, file); // 8MB buffer

        tracing::debug!("Created repository: {}", path.display());

        Ok(Self {
            session_id: filename.to_string(),
            path,
            writer,
            current_offset: 0,
            _phantom: PhantomData,
        })
    }

    /// Open an existing file repository for appending.
    pub fn open(base_dir: impl AsRef<Path>, filename: impl AsRef<str>) -> Result<Self> {
        let base_dir = base_dir.as_ref();
        let filename = filename.as_ref();
        let path = base_dir.join(filename);

        let file = OpenOptions::new()
            .append(true)
            .open(&path)
            .map_err(RepositoryError::Io)?;

        let current_offset = file.metadata().map_err(RepositoryError::Io)?.len();
        let writer = BufWriter::with_capacity(8 * 1024 * 1024, file);

        tracing::debug!(
            "Opened repository: {} at offset {}",
            path.display(),
            current_offset
        );

        Ok(Self {
            session_id: filename.to_string(),
            path,
            writer,
            current_offset,
            _phantom: PhantomData,
        })
    }

    /// Open or create a file repository.
    ///
    /// Creates the directory and file if they don't exist, or opens existing file for appending.
    pub fn open_or_create(base_dir: impl AsRef<Path>, filename: impl AsRef<str>) -> Result<Self> {
        let base_dir = base_dir.as_ref();
        std::fs::create_dir_all(base_dir).map_err(RepositoryError::Io)?;

        let filename = filename.as_ref();
        let path = base_dir.join(filename);

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(RepositoryError::Io)?;

        let current_offset = file.metadata().map_err(RepositoryError::Io)?.len();
        let writer = BufWriter::with_capacity(8 * 1024 * 1024, file);

        tracing::debug!(
            "Opened/created repository: {} at offset {}",
            path.display(),
            current_offset
        );

        Ok(Self {
            session_id: filename.to_string(),
            path,
            writer,
            current_offset,
            _phantom: PhantomData,
        })
    }

    /// Append an item to the log.
    ///
    /// Returns the byte offset where the item was written.
    pub fn append(&mut self, item: &T) -> Result<u64> {
        let offset = self.current_offset;

        // Serialize item
        let bytes =
            bincode::serialize(item).map_err(|e| RepositoryError::Serialization(e.to_string()))?;
        let len = bytes.len() as u32;

        // Write [length][data]
        self.writer
            .write_all(&len.to_le_bytes())
            .map_err(RepositoryError::Io)?;
        self.writer.write_all(&bytes).map_err(RepositoryError::Io)?;

        self.current_offset += 4 + bytes.len() as u64;

        Ok(offset)
    }

    /// Read an item at a specific byte offset.
    ///
    /// Returns `None` if the offset is beyond the end of the file.
    /// Returns `Some((item, next_offset))` where next_offset is the byte position after this entry.
    pub fn read_at_offset(&self, byte_offset: u64) -> Result<Option<(T, u64)>> {
        // Open a separate reader (doesn't interfere with writer)
        let file = File::open(&self.path).map_err(RepositoryError::Io)?;
        let file_size = file.metadata().map_err(RepositoryError::Io)?.len();

        // Check if offset is beyond file
        if byte_offset >= file_size {
            return Ok(None);
        }

        let mut reader = BufReader::new(file);

        // Seek to offset
        reader
            .seek(SeekFrom::Start(byte_offset))
            .map_err(RepositoryError::Io)?;

        // Read length
        let mut len_bytes = [0u8; 4];
        reader
            .read_exact(&mut len_bytes)
            .map_err(RepositoryError::Io)?;
        let len = u32::from_le_bytes(len_bytes) as usize;

        // Read data
        let mut data = vec![0u8; len];
        reader.read_exact(&mut data).map_err(RepositoryError::Io)?;

        // Deserialize
        let item = bincode::deserialize(&data)
            .map_err(|e| RepositoryError::Serialization(e.to_string()))?;

        // Calculate next offset: current + 4 bytes (length prefix) + data length
        let next_offset = byte_offset + 4 + len as u64;

        Ok(Some((item, next_offset)))
    }

    /// Flush buffered writes to disk.
    pub fn flush(&mut self) -> Result<()> {
        self.writer.flush().map_err(RepositoryError::Io)?;
        Ok(())
    }

    /// Get the current size of the log in bytes.
    pub fn size(&self) -> Result<u64> {
        Ok(self.current_offset)
    }

    /// Get the session ID (filename).
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Get the file path.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl<T> Drop for FileRepository<T> {
    fn drop(&mut self) {
        if let Err(e) = self.writer.flush() {
            tracing::warn!(
                "Failed to flush repository '{}' on drop: {}",
                self.session_id,
                e
            );
        }
    }
}

// ============================================================================
// Trait Implementations
// ============================================================================

impl EventRepository for FileRepository<Event> {
    fn append(&mut self, event: &Event) -> Result<u64> {
        self.append(event)
    }

    fn read_at_offset(&self, byte_offset: u64) -> Result<Option<(Event, u64)>> {
        self.read_at_offset(byte_offset)
    }

    fn flush(&mut self) -> Result<()> {
        self.flush()
    }

    fn size(&self) -> Result<u64> {
        self.size()
    }

    fn session_id(&self) -> &str {
        self.session_id()
    }
}

impl ActionLogWriter for FileRepository<ActionLogEntry> {
    fn append(&mut self, entry: &ActionLogEntry) -> Result<u64> {
        self.append(entry)
    }

    fn flush(&mut self) -> Result<()> {
        self.flush()
    }

    fn size(&self) -> Result<u64> {
        self.size()
    }

    fn session_id(&self) -> &str {
        self.session_id()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use tempfile::TempDir;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestItem {
        id: u64,
        name: String,
    }

    #[test]
    fn test_create_and_append() {
        let temp_dir = TempDir::new().unwrap();
        let mut repo = FileRepository::<TestItem>::create(temp_dir.path(), "test.log").unwrap();

        let item1 = TestItem {
            id: 1,
            name: "first".to_string(),
        };
        let item2 = TestItem {
            id: 2,
            name: "second".to_string(),
        };

        let offset1 = repo.append(&item1).unwrap();
        let offset2 = repo.append(&item2).unwrap();

        assert_eq!(offset1, 0);
        assert!(offset2 > 0);

        repo.flush().unwrap();
    }

    #[test]
    fn test_read_at_offset() {
        let temp_dir = TempDir::new().unwrap();
        let mut repo = FileRepository::<TestItem>::create(temp_dir.path(), "test.log").unwrap();

        let item1 = TestItem {
            id: 1,
            name: "first".to_string(),
        };
        let item2 = TestItem {
            id: 2,
            name: "second".to_string(),
        };

        let offset1 = repo.append(&item1).unwrap();
        let offset2 = repo.append(&item2).unwrap();
        repo.flush().unwrap();

        // Read items back
        let (read1, next_offset1) = repo.read_at_offset(offset1).unwrap().unwrap();
        let (read2, next_offset2) = repo.read_at_offset(offset2).unwrap().unwrap();

        assert_eq!(read1, item1);
        assert_eq!(read2, item2);
        assert_eq!(next_offset1, offset2);
        assert!(next_offset2 > offset2);

        // Read beyond end
        let read_end = repo.read_at_offset(999999).unwrap();
        assert_eq!(read_end, None);
    }

    #[test]
    fn test_open_existing() {
        let temp_dir = TempDir::new().unwrap();

        // Create and write
        {
            let mut repo = FileRepository::<TestItem>::create(temp_dir.path(), "test.log").unwrap();
            let item = TestItem {
                id: 1,
                name: "test".to_string(),
            };
            repo.append(&item).unwrap();
            repo.flush().unwrap();
        }

        // Open and append more
        {
            let mut repo = FileRepository::<TestItem>::open(temp_dir.path(), "test.log").unwrap();
            let item2 = TestItem {
                id: 2,
                name: "second".to_string(),
            };
            let offset2 = repo.append(&item2).unwrap();
            repo.flush().unwrap();

            assert!(offset2 > 0);
        }
    }
}
