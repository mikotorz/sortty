use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::State;

/// Shared cancellation signal for the currently in-flight scan or apply.
/// One flag is enough — the frontend only ever allows one of either to run
/// at a time (the Scan/Apply buttons are disabled while busy), so there's no
/// need to track cancellation per-request-id.
#[derive(Clone, Default)]
pub struct CancelFlag(Arc<AtomicBool>);

impl CancelFlag {
    pub fn reset(&self) {
        self.0.store(false, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

#[tauri::command]
pub fn cancel_current_operation(state: State<'_, CancelFlag>) {
    state.0.store(true, Ordering::SeqCst);
}
