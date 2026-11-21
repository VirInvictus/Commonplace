//! File watching (spec row 72): `notify` 8.x behind
//! `notify-debouncer-full`, app-crate-only. The watcher decides WHEN
//! to reconcile, never WHAT changed: every coalesced event batch maps
//! to one "the folder may have moved" signal, and the Phase 4
//! reconcile pass (path + re-stat keying) is the truth that runs on
//! it. The module is GTK-free so the watch behavior tests headless.

use std::path::Path;
use std::time::Duration;

use notify::RecursiveMode;
use notify_debouncer_full::notify::RecommendedWatcher;
use notify_debouncer_full::{Debouncer, RecommendedCache, new_debouncer};

/// The coalesced signal a consumer acts on. Events carry no payload
/// on purpose: the reconcile rescans and diffs by path, so an event
/// batch is only ever "look again".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FolderMoved;

/// The handle: dropping it stops watching. The debouncer must stay
/// alive for the watcher to run (notify's watch is bound to it).
pub struct WatchHandle {
    _debouncer: Debouncer<RecommendedWatcher, RecommendedCache>,
}

/// Watch `folder` recursively; every debounced event batch (files
/// created, written, renamed, removed) sends [`FolderMoved`] to `tx`.
/// The debounce window coalesces the Create+Rename pairs that
/// atomic-rename saves arrive as (crates §2), so one logical save is
/// one signal.
pub fn spawn(
    folder: &Path,
    debounce: Duration,
    on_move: impl Fn(FolderMoved) + Send + 'static,
) -> Result<WatchHandle, notify::Error> {
    let mut debouncer: Debouncer<RecommendedWatcher, RecommendedCache> = new_debouncer(
        debounce,
        None,
        move |result: notify_debouncer_full::DebounceEventResult| {
            if let Ok(events) = result
                && events.iter().any(|event| !event.event.kind.is_access())
            {
                on_move(FolderMoved);
            }
        },
    )?;
    debouncer.watch(Path::new(folder), RecursiveMode::Recursive)?;
    // Root-change reconfiguration (upstream's FSEvents root watcher,
    // map §1.1): notify errors when the watched root disappears; the
    // reconcile's missing-folder path handles that on the consumer
    // side.
    let _ = debouncer.watch(
        Path::new(folder.parent().unwrap_or(folder)),
        RecursiveMode::NonRecursive,
    );
    Ok(WatchHandle {
        _debouncer: debouncer,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::mpsc;
    use std::time::Instant;

    fn scratch(tag: &str) -> PathBuf {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "commonplace-watch-{}-{}-{}",
            tag,
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn recv_signal(rx: &mpsc::Receiver<FolderMoved>, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return false;
            }
            match rx.recv_timeout(remaining.min(Duration::from_millis(100))) {
                Ok(FolderMoved) => return true,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => return false,
            }
        }
    }

    #[test]
    fn external_write_signals_reconcile() {
        let dir = scratch("write");
        let (tx, rx) = mpsc::channel();
        let _handle = spawn(&dir, Duration::from_millis(100), move |moved| {
            let _ = tx.send(moved);
        })
        .unwrap();

        // Give inotify a beat to arm, then write like an external
        // editor would: temp file + rename (the atomic-save shape).
        std::thread::sleep(Duration::from_millis(150));
        let tmp = dir.join(".tmp-note");
        fs::write(&tmp, "content\n").unwrap();
        fs::rename(&tmp, dir.join("Note.md")).unwrap();

        assert!(
            recv_signal(&rx, Duration::from_secs(5)),
            "the write signaled"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn external_rename_and_remove_signal_reconcile() {
        let dir = scratch("rename");
        fs::write(dir.join("A.md"), "a\n").unwrap();
        let (tx, rx) = mpsc::channel();
        let _handle = spawn(&dir, Duration::from_millis(100), move |moved| {
            let _ = tx.send(moved);
        })
        .unwrap();
        std::thread::sleep(Duration::from_millis(150));

        fs::rename(dir.join("A.md"), dir.join("B.md")).unwrap();
        assert!(
            recv_signal(&rx, Duration::from_secs(5)),
            "the rename signaled"
        );

        fs::remove_file(dir.join("B.md")).unwrap();
        assert!(
            recv_signal(&rx, Duration::from_secs(5)),
            "the remove signaled"
        );
        fs::remove_dir_all(&dir).unwrap();
    }
}
