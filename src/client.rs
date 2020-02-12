use std::mem::size_of;
use std::os::raw::{c_int};
use std::sync::atomic::{AtomicUsize, Ordering};

use super::bindings::{
    cef_base_ref_counted_t, cef_client_t, cef_context_menu_handler_t, cef_display_handler_t,
    cef_life_span_handler_t, cef_request_handler_t, cef_browser_t, cef_frame_t, cef_process_id_t,
    cef_process_message_t, cef_string_userfree_t, cef_string_userfree_utf16_free, cef_string_t
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

unsafe extern "C" fn on_process_message_received(
    _slf: *mut cef_client_t,
    browser: *mut cef_browser_t,
    frame: *mut cef_frame_t,
    _source_process: cef_process_id_t,
    message: *mut cef_process_message_t,
) -> c_int {
    let cef_message_name: cef_string_userfree_t = ((*message).get_name.expect("get_name is a function"))(message);
    let chars: *mut u16 = (*cef_message_name).str;
    let len: usize = (*cef_message_name).length as usize;
    let chars = std::slice::from_raw_parts(chars, len);
    let message_name = std::char::decode_utf16(chars.iter().cloned())
        .map(|r| r.unwrap_or(std::char::REPLACEMENT_CHARACTER))
        .collect::<String>();
    cef_string_userfree_utf16_free(cef_message_name);

    if message_name == "print_to_pdf" {
        // get the path
        let args = ((*message).get_argument_list.expect("get_argument_list is a function"))(message);
        let cef_path: cef_string_userfree_t = ((*args).get_string.expect("get_string is a function"))(args, 0);
        let chars: *mut u16 = (*cef_path).str;
        let len: usize = (*cef_path).length as usize;
        let chars = std::slice::from_raw_parts(chars, len);
        let path = std::char::decode_utf16(chars.iter().cloned())
            .map(|r| r.unwrap_or(std::char::REPLACEMENT_CHARACTER))
            .collect::<String>();
        cef_string_userfree_utf16_free(cef_path);

        super::browser::Browser::print_to_pdf_pointer(browser, path, Some(Box::from(move |ok| {
            // now send an IPC message back to the renderer
            // convert the message name to a CEF string
            let mut cef_message_name = cef_string_t::default();
            let message_name = "print_to_pdf_done".as_bytes();
            let message_name = std::ffi::CString::new(message_name).unwrap();
            super::bindings::cef_string_utf8_to_utf16(message_name.as_ptr(), message_name.to_bytes().len() as u64, &mut cef_message_name);

            // build the message
            let message = super::bindings::cef_process_message_create(&cef_message_name);
            let args = ((*message).get_argument_list.expect("get_argument_list is a function"))(message);
            ((*args).set_size.expect("set_size is a function"))(args, 1);
            ((*args).set_bool.expect("set_bool is a function"))(args, 0, ok as i32);

            // send the message
            ((*frame).send_process_message.expect("send_process_message is a function"))(frame, super::bindings::cef_process_id_t_PID_RENDERER, message);
        })));

        1
    }
    else {
        0
    }
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
            on_process_message_received: Some(on_process_message_received),
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
