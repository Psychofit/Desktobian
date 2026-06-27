//! Small shared helpers.

use std::sync::atomic::{AtomicBool, Ordering};

static TERMINATE: AtomicBool = AtomicBool::new(false);

/// Install handlers for SIGINT/SIGTERM so the backends can shut down cleanly
/// (free mpv, tear down surfaces) instead of being killed abruptly.
pub fn install_signal_handlers() {
    extern "C" fn handle(_sig: std::os::raw::c_int) {
        TERMINATE.store(true, Ordering::SeqCst);
    }
    // SAFETY: `handle` is async-signal-safe (it only stores to an atomic).
    unsafe {
        libc::signal(libc::SIGINT, handle as *const () as usize);
        libc::signal(libc::SIGTERM, handle as *const () as usize);
    }
}

/// Whether a termination signal has been received.
pub fn should_terminate() -> bool {
    TERMINATE.load(Ordering::SeqCst)
}
