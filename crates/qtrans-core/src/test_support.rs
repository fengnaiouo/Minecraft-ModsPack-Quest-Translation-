//! Helpers shared by unit tests.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Creates a unique temporary directory for a test.
///
/// The process ID and a counter keep parallel tests apart even when the
/// system clock has a coarse resolution.
pub(crate) fn unique_temp_dir(label: &str) -> PathBuf {
    let base = std::env::temp_dir();
    for _ in 0..1000 {
        let dir = base.join(format!(
            "qtrans-{label}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        match std::fs::create_dir(&dir) {
            Ok(()) => return dir,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => panic!("create test temp dir {}: {error}", dir.display()),
        }
    }
    panic!("could not create a unique test temp dir");
}
