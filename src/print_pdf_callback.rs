use std::mem::size_of;
use std::os::raw::{c_int};
use std::sync::atomic::{AtomicUsize, Ordering};
use super::bindings::{
    cef_base_ref_counted_t, _cef_pdf_print_callback_t, cef_string_t,
};

#[repr(C)]
pub struct PDFPrintCallback {
    pdf_print_callback: _cef_pdf_print_callback_t,
    ref_count: AtomicUsize,
}

unsafe extern "C" fn on_pdf_print_finished(_slf: *mut _cef_pdf_print_callback_t, _path: *const cef_string_t, ok: c_int) {
    log::debug!("printing done, ok: {}", ok);
}

pub fn allocate() -> *mut PDFPrintCallback {
    let handler = PDFPrintCallback {
        pdf_print_callback: _cef_pdf_print_callback_t {
            base: cef_base_ref_counted_t {
                size: size_of::<PDFPrintCallback>() as u64,
                add_ref: Some(add_ref_pdf_print_callback),
                release: Some(release_pdf_print_callback),
                has_one_ref: Some(has_one_ref_pdf_print_callback),
                has_at_least_one_ref: Some(has_at_least_one_ref_pdf_print_callback),
            },
            on_pdf_print_finished: Some(on_pdf_print_finished),
        },
        ref_count: AtomicUsize::new(1),
    };

    Box::into_raw(Box::from(handler))
}

extern "C" fn add_ref_pdf_print_callback(base: *mut cef_base_ref_counted_t) {
    let life_span_handler = base as *mut PDFPrintCallback;
    unsafe {
        (*life_span_handler)
            .ref_count
            .fetch_add(1, Ordering::SeqCst);
    }
}

extern "C" fn release_pdf_print_callback(base: *mut cef_base_ref_counted_t) -> c_int {
    let life_span_handler = base as *mut PDFPrintCallback;
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

extern "C" fn has_one_ref_pdf_print_callback(base: *mut cef_base_ref_counted_t) -> c_int {
    let life_span_handler = base as *mut PDFPrintCallback;
    let count = unsafe { (*life_span_handler).ref_count.load(Ordering::SeqCst) };
    if count == 1 {
        1
    } else {
        0
    }
}

extern "C" fn has_at_least_one_ref_pdf_print_callback(base: *mut cef_base_ref_counted_t) -> c_int {
    let life_span_handler = base as *mut PDFPrintCallback;
    let count = unsafe { (*life_span_handler).ref_count.load(Ordering::SeqCst) };
    if count >= 1 {
        1
    } else {
        0
    }
}