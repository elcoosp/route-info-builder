//! Test-only helpers shared across the crate's unit tests.

#![cfg(test)]

use std::sync::{Mutex, MutexGuard, OnceLock};

static CWD_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

/// Serializes tests that change the process working directory and restores
/// the previous directory when dropped.
pub(crate) struct CurrentDirGuard {
    _lock: MutexGuard<'static, ()>,
    previous: std::path::PathBuf,
}

impl CurrentDirGuard {
    /// Acquires the working-directory lock and remembers the current dir.
    ///
    /// # Errors
    ///
    /// Returns the [`std::io::Error`] from reading the current directory.
    pub(crate) fn lock() -> std::io::Result<Self> {
        let mutex = CWD_LOCK.get_or_init(|| Mutex::new(()));
        let guard = mutex
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let previous = std::env::current_dir()?;
        Ok(Self {
            _lock: guard,
            previous,
        })
    }
}

impl Drop for CurrentDirGuard {
    fn drop(&mut self) {
        let _unused = std::env::set_current_dir(&self.previous);
    }
}
