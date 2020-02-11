use std::mem::size_of;
use std::os::raw::{c_int};
use std::sync::atomic::{AtomicUsize, Ordering};
use super::schedule::Schedule;
use std::sync::Arc;

use super::bindings::{
    cef_base_ref_counted_t, cef_browser_process_handler_t,
};

#[repr(C)]
pub struct BrowserProcessHandler {
    handler: cef_browser_process_handler_t,
    ref_count: AtomicUsize,
    schedule: Arc<Schedule>,
}

impl BrowserProcessHandler {
    pub fn inc_ref(&self) {
        self.ref_count.fetch_add(1, Ordering::SeqCst);
    }
}

unsafe extern "C" fn on_schedule_message_pump_work(slf: *mut cef_browser_process_handler_t, delay_ms: i64) {
    //log::debug!("on_schedule_message_pump_work, delay: {}", delay_ms);
    let handler = slf as *mut BrowserProcessHandler;
    let handler = &(*handler);
    handler.schedule.schedule_work(delay_ms);
}

pub fn allocate(schedule: Arc<Schedule>) -> *mut BrowserProcessHandler {
    let handler = BrowserProcessHandler {
        handler: cef_browser_process_handler_t {
            base: cef_base_ref_counted_t {
                size: size_of::<BrowserProcessHandler>() as u64,
                add_ref: Some(add_ref),
                release: Some(release),
                has_one_ref: Some(has_one_ref),
                has_at_least_one_ref: Some(has_at_least_one_ref),
            },
            on_context_initialized: None,
            on_before_child_process_launch: None,
            on_render_process_thread_created: None,
            get_print_handler: None,
            on_schedule_message_pump_work: Some(on_schedule_message_pump_work),
        },
        ref_count: AtomicUsize::new(1),
        schedule,
    };

    Box::into_raw(Box::from(handler))
}

extern "C" fn add_ref(base: *mut cef_base_ref_counted_t) {
    let life_span_handler = base as *mut BrowserProcessHandler;
    unsafe {
        (*life_span_handler)
            .ref_count
            .fetch_add(1, Ordering::SeqCst);
    }
}

extern "C" fn release(base: *mut cef_base_ref_counted_t) -> c_int {
    let life_span_handler = base as *mut BrowserProcessHandler;
    let count = unsafe {
        (*life_span_handler)
            .ref_count
            .fetch_sub(1, Ordering::SeqCst)
            - 1
    };

    if count == 0 {
        unsafe {
            Box::from_raw(life_span_handler);
        }
        1
    } else {
        0
    }
}

extern "C" fn has_one_ref(base: *mut cef_base_ref_counted_t) -> c_int {
    let life_span_handler = base as *mut BrowserProcessHandler;
    let count = unsafe { (*life_span_handler).ref_count.load(Ordering::SeqCst) };
    if count == 1 {
        1
    } else {
        0
    }
}

extern "C" fn has_at_least_one_ref(base: *mut cef_base_ref_counted_t) -> c_int {
    let life_span_handler = base as *mut BrowserProcessHandler;
    let count = unsafe { (*life_span_handler).ref_count.load(Ordering::SeqCst) };
    if count >= 1 {
        1
    } else {
        0
    }
}
