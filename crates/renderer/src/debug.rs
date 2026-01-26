//! Debug helpers and logging flags.

use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};

/// Renderer logging flags.
#[derive(Clone, Debug, Default)]
pub struct AwsmRendererLogging {
    pub render_timings: bool,
}

/// Debug ID reserved for renderable tracking.
pub const DEBUG_ID_RENDERABLE: u32 = u32::MAX - 1;

static DEBUG_TRANSACTION_ID: LazyLock<Mutex<HashMap<u32, u64>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static DEBUG_UNIQUE_STRING: LazyLock<Mutex<HashMap<u32, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

// returns the old value
fn bump_transaction_count(id: u32) -> u64 {
    let mut lock = DEBUG_TRANSACTION_ID.lock().unwrap();
    let value = lock.entry(id).or_insert(0);
    let curr = *value;
    *value += 1;

    curr
}

/// Runs a closure only once per debug ID.
pub fn debug_once(id: u32, f: impl FnOnce()) {
    let transaction_count = bump_transaction_count(id);

    if transaction_count == 0 {
        f();
    }
}

/// Runs a closure up to `n` times per debug ID.
pub fn debug_n(id: u32, n: u64, f: impl FnOnce()) {
    let transaction_count = bump_transaction_count(id);

    if transaction_count < n {
        f();
    }
}

/// Runs a closure if the input string changes for the debug ID.
pub fn debug_unique_string(id: u32, input: &str, f: impl FnOnce()) {
    bump_transaction_count(id);

    let mut lock = DEBUG_UNIQUE_STRING.lock().unwrap();
    if let Some(value) = lock.get(&id) {
        if value == input {
            return; // already set
        }
    }

    f();
    lock.insert(id, input.to_string());
}
