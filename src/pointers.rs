use std::ptr::null;
use std::time::Duration;
use crate::debugging::run_every;

pub unsafe fn jump_pointers<T>(mut base_ptr: *const usize, jumps: &[usize]) -> *const T {
    for (i, jump) in jumps.iter().enumerate() {
        if base_ptr.is_null() {
            return null();
        }

        if base_ptr as usize % 8 != 0 {
            run_every!("bad pointer address" every Duration::from_secs(1) => {
                tracing::error!("Invalid pointer after {i} jumps: {base_ptr:#?}");
            });
            return null();
        }

        base_ptr = (*base_ptr + jump) as *const usize;
    }

    base_ptr.cast()
}