use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};

#[derive(Clone, Debug, Default)]
pub struct AwsmRendererLogging {
    pub render_timings: bool,
}

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

pub fn debug_once(id: u32, f: impl FnOnce()) {
    let transaction_count = bump_transaction_count(id);

    if transaction_count == 0 {
        f();
    }
}

pub fn debug_n(id: u32, n: u64, f: impl FnOnce()) {
    let transaction_count = bump_transaction_count(id);

    if transaction_count < n {
        f();
    }
}

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
