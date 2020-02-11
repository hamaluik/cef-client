use std::mem::size_of;
use std::os::raw::{c_int};
use std::sync::atomic::{AtomicUsize, Ordering};

use super::bindings::{
    cef_base_ref_counted_t, cef_client_t, cef_context_menu_handler_t, cef_display_handler_t,
    cef_life_span_handler_t, cef_request_handler_t,
};
use super::context_menu_handler::{self, ContextMenuHandler};
use super::display_handler::{self, DisplayHandler};
use super::life_span_handler::{self, LifeSpanHandler};
use super::request_handler::{self, RequestHandler};

#[repr(C)]
pub struct Client {
    client: cef_client_t,
    ref_count: AtomicUsize,
    life_span_handler: *mut LifeSpanHandler,
    context_menu_handler: *mut ContextMenuHandler,
    request_handler: *mut RequestHandler,
    display_handler: *mut DisplayHandler,
}

extern "C" fn get_life_span_handler(slf: *mut cef_client_t) -> *mut cef_life_span_handler_t {
    let client = slf as *mut Client;
    let handler = unsafe { (*client).life_span_handler };
    unsafe { (*handler).inc_ref() };
    handler as *mut cef_life_span_handler_t
}

extern "C" fn get_context_menu_handler(slf: *mut cef_client_t) -> *mut cef_context_menu_handler_t {
    let client = slf as *mut Client;
    let handler = unsafe { (*client).context_menu_handler };
    unsafe { (*handler).inc_ref() };
    handler as *mut cef_context_menu_handler_t
}

extern "C" fn get_request_handler(slf: *mut cef_client_t) -> *mut cef_request_handler_t {
    let client = slf as *mut Client;
    let handler = unsafe { (*client).request_handler };
    unsafe { (*handler).inc_ref() };
    handler as *mut cef_request_handler_t
}

extern "C" fn get_display_handler(slf: *mut cef_client_t) -> *mut cef_display_handler_t {
    let client = slf as *mut Client;
    let handler = unsafe { (*client).display_handler };
    unsafe { (*handler).inc_ref() };
    handler as *mut cef_display_handler_t
}

pub fn allocate() -> *mut Client {
    let client = Client {
        client: cef_client_t {
            base: cef_base_ref_counted_t {
                size: size_of::<Client>() as u64,
                add_ref: Some(add_ref),
                release: Some(release),
                has_one_ref: Some(has_one_ref),
                has_at_least_one_ref: Some(has_at_least_one_ref),
            },
            get_context_menu_handler: Some(get_context_menu_handler),
            get_dialog_handler: None,
            get_display_handler: Some(get_display_handler),
            get_download_handler: None,
            get_drag_handler: None,
            get_find_handler: None,
            get_focus_handler: None,
            get_jsdialog_handler: None,
            get_keyboard_handler: None,
            get_life_span_handler: Some(get_life_span_handler),
            get_load_handler: None,
            get_render_handler: None,
            get_request_handler: Some(get_request_handler),
            on_process_message_received: None,
        },
        ref_count: AtomicUsize::new(1),
        life_span_handler: life_span_handler::allocate(),
        context_menu_handler: context_menu_handler::allocate(),
        request_handler: request_handler::allocate(),
        display_handler: display_handler::allocate(),
    };

    Box::into_raw(Box::from(client))
}

pub unsafe fn set_fullscreen_listener<F: FnMut(bool) + 'static>(slf: *mut Client, listener: F) {
    let client = slf as *mut Client;
    super::display_handler::set_fullscreen_listener((*client).display_handler, listener);
}

extern "C" fn add_ref(base: *mut cef_base_ref_counted_t) {
    let client = base as *mut Client;
    unsafe {
        (*client).ref_count.fetch_add(1, Ordering::SeqCst);
    }
}

extern "C" fn release(base: *mut cef_base_ref_counted_t) -> c_int {
    let client = base as *mut Client;
    let count = unsafe { (*client).ref_count.fetch_sub(1, Ordering::SeqCst) - 1 };

    if count == 0 {
        unsafe {
            Box::from_raw(client);
            // TODO: free our handlers here too?
        }
        1
    } else {
        0
    }
}

extern "C" fn has_one_ref(base: *mut cef_base_ref_counted_t) -> c_int {
    let client = base as *mut Client;
    let count = unsafe { (*client).ref_count.load(Ordering::SeqCst) };
    if count == 1 {
        1
    } else {
        0
    }
}

extern "C" fn has_at_least_one_ref(base: *mut cef_base_ref_counted_t) -> c_int {
    let client = base as *mut Client;
    let count = unsafe { (*client).ref_count.load(Ordering::SeqCst) };
    if count >= 1 {
        1
    } else {
        0
    }
}
