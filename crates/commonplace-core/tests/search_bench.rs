//! The 10k-note filter benchmark with its recorded latency gate.
//!
//! The Linux ecosystem brief (§3) found no published note-app number
//! to borrow: "Commonplace must measure its own." This test measures,
//! prints, and gates. It is skipped by default locally (like the
//! Phase 1 load stub); Phase 20 wires the gates into CI.
//!
//! Run: `cargo test -p commonplace-core --test search_bench -- --ignored`

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant, SystemTime};

use commonplace_core::search::{FilterState, SortOptions};
use commonplace_core::store::load_folder;

/// The recorded gate (spec §5 row 16's "instant" made checkable): no
/// single keystroke in the measured typing sequence may take longer.
/// The first keystroke is the cold pass (folding 10k bodies); the
/// rest are resume passes over shrinking candidate sets. Benchmarks
/// measure optimized builds: the recorded 50 ms gate applies to
/// `--release`; a debug build gets a wide sanity gate so an accidental
/// debug run fails loudly for the right reason.
const RELEASE_KEYSTROKE_BUDGET: Duration = Duration::from_millis(50);
const DEBUG_KEYSTROKE_BUDGET: Duration = Duration::from_millis(500);

/// A scratch notes folder under the OS temp dir, removed on drop.
struct TempFolder {
    path: PathBuf,
}

impl TempFolder {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let unique = format!(
            "commonplace-bench-{}-{}-{}-{}",
            tag,
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        );
        let path = std::env::temp_dir().join(unique);
        fs::create_dir_all(&path).unwrap();
        TempFolder { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempFolder {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
#[ignore = "opt-in 10k filter benchmark"]
fn filter_10k_notes_per_keystroke_latency_gate() {
    let folder = TempFolder::new("filter10k");
    for i in 0..10_000 {
        let mut body = format!("Note number {i:05}\nbody line for note {i:05}");
        if i % 50 == 0 {
            body.push_str(" notational velocity keeps every keystroke instant");
        }
        fs::write(
            folder.path().join(format!("note-{i:05}.md")),
            format!("{body}\n"),
        )
        .unwrap();
    }

    let load_start = Instant::now();
    let outcome = load_folder(folder.path()).unwrap();
    println!("cold load: {:?}", load_start.elapsed());
    assert_eq!(outcome.store.len(), 10_000);
    let notes = outcome.store.notes();

    let mut state = FilterState::new(SortOptions::default());
    let keystroke_budget = if cfg!(debug_assertions) {
        DEBUG_KEYSTROKE_BUDGET
    } else {
        RELEASE_KEYSTROKE_BUDGET
    };
    let sequence = "notational velocity keeps";
    let mut worst = Duration::ZERO;
    let mut total = Duration::ZERO;
    let mut last_visible = 0usize;
    for end in 1..=sequence.len() {
        let raw = &sequence[..end];
        let start = Instant::now();
        let visible = state.update(raw, notes).unwrap();
        let elapsed = start.elapsed();
        worst = worst.max(elapsed);
        total += elapsed;
        last_visible = visible.len();
        println!("{raw:?}: {} notes in {elapsed:?}", visible.len());
        assert!(
            elapsed <= keystroke_budget,
            "keystroke {raw:?} took {elapsed:?}, over the {keystroke_budget:?} gate"
        );
    }
    // The match set is real: the body-marked notes survive to the end.
    assert_eq!(last_visible, 200);
    // And the visible order is the sorted stable order.
    assert!(
        state
            .visible()
            .windows(2)
            .all(|pair| notes[pair[0]].mtime >= notes[pair[1]].mtime)
    );
    println!(
        "worst keystroke: {:?} (gate {keystroke_budget:?}), total {total:?} over {} keystrokes",
        worst,
        sequence.len()
    );
}
