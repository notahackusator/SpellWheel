use lazy_static::lazy_static;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

lazy_static!(
    static ref USE_QUICK_ITEM: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
);

pub fn queue_use_quick_item() {
    USE_QUICK_ITEM.store(true, Ordering::Relaxed);
}

pub fn poll_use_quick_item() -> bool {
    USE_QUICK_ITEM.fetch_and(false, Ordering::Relaxed)
}