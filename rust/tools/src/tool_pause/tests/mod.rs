use super::*;

use serial_test::serial;

use std::sync::atomic::{AtomicBool, Ordering};

fn with_clean_handler<T>(f: impl FnOnce(Uuid) -> T) -> T {
    let sid = Uuid::new_v4();
    set_pause_handler(sid, None);
    let result = f(sid);
    set_pause_handler(sid, None);
    result
}

mod part_1;
