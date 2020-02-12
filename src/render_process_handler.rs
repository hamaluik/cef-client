use std::mem::size_of;
use std::os::raw::{c_int};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::ffi::CString;

use super::bindings::{
    cef_base_ref_counted_t, cef_render_process_handler_t,
    cef_browser_t, cef_frame_t, cef_v8context_t, cef_string_t, cef_string_utf8_to_utf16,
    cef_register_extension, cef_v8handler_t, cef_process_id_t, cef_process_message_t,
    cef_string_userfree_t, cef_string_userfree_utf16_free,
};
use super::v8_pdf_print_handler::{self, V8PDFPrintHandler};

#[repr(C)]
pub struct RenderProcessHandler {
    render_process_handler: cef_render_process_handler_t,
    ref_count: AtomicUsize,
    pdf_print_extension: *mut V8PDFPrintHandler,
}

impl RenderProcessHandler {
    pub fn inc_ref(&self) {
        self.ref_count.fetch_add(1, Ordering::SeqCst);
    }
}

unsafe extern "C" fn on_web_kit_initialized(slf: *mut cef_render_process_handler_t) {
    // TODO: register extension?
    let code = super::v8_pdf_print_handler::CODE.as_bytes();
    let code = CString::new(code).unwrap();
    let mut cef_code = cef_string_t::default();
    cef_string_utf8_to_utf16(code.as_ptr(), code.to_bytes().len() as u64, &mut cef_code);

    let extension_name = "CEF PDF Printer";
    let extension_name = extension_name.as_bytes();
    let extension_name = CString::new(extension_name).unwrap();
    let mut cef_extension_name = cef_string_t::default();
    cef_string_utf8_to_utf16(extension_name.as_ptr(), extension_name.to_bytes().len() as u64, &mut cef_extension_name);

    let render_process_handler = slf as *mut RenderProcessHandler;
    let extension = (*render_process_handler).pdf_print_extension;
    cef_register_extension(&cef_extension_name, &cef_code, extension as *mut cef_v8handler_t);
    log::debug!("registered pdf printer extension");
}

unsafe extern "C" fn on_context_created(slf: *mut cef_render_process_handler_t, _browser: *mut cef_browser_t, frame: *mut cef_frame_t, _context: *mut cef_v8context_t) {
    // store the frame on our extension handler so it can send an IPC message
    let _self = slf as *mut RenderProcessHandler;
    (*(*_self).pdf_print_extension).frame = Some(frame);
}

unsafe extern "C" fn on_process_message_received(
    slf: *mut cef_render_process_handler_t,
    _browser: *mut cef_browser_t,
    _frame: *mut cef_frame_t,
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

    if message_name == "print_to_pdf_done" {
        let args = ((*message).get_argument_list.expect("get_argument_list is a function"))(message);
        let ok: bool = ((*args).get_bool.expect("get_bool is a function"))(args, 0) == 1;
        super::v8_pdf_print_handler::on_pdf_print_done((*(slf as *mut RenderProcessHandler)).pdf_print_extension, ok);
        1
    }
    else {
        log::warn!("unhandled process message in renderer: `{}`", message_name);
        0
    }
}

pub fn allocate() -> *mut RenderProcessHandler {
    let handler = RenderProcessHandler {
        render_process_handler: cef_render_process_handler_t {
            base: cef_base_ref_counted_t {
                size: size_of::<RenderProcessHandler>() as u64,
                add_ref: Some(add_ref),
                release: Some(release),
                has_one_ref: Some(has_one_ref),
                has_at_least_one_ref: Some(has_at_least_one_ref),
            },
            on_render_thread_created: None,
            on_web_kit_initialized: Some(on_web_kit_initialized),
            on_browser_created: None,
            on_browser_destroyed: None,
            get_load_handler: None,
            on_context_created: Some(on_context_created),
            on_context_released: None,
            on_uncaught_exception: None,
            on_focused_node_changed: None,
            on_process_message_received: Some(on_process_message_received),
        },
        ref_count: AtomicUsize::new(1),
        pdf_print_extension: v8_pdf_print_handler::allocate(),
    };

    Box::into_raw(Box::from(handler))
}

extern "C" fn add_ref(base: *mut cef_base_ref_counted_t) {
    let render_process_handler = base as *mut RenderProcessHandler;
    unsafe { (*render_process_handler).ref_count.fetch_add(1, Ordering::SeqCst) };
}

extern "C" fn release(base: *mut cef_base_ref_counted_t) -> c_int {
    let render_process_handler = base as *mut RenderProcessHandler;
    let count = unsafe { (*render_process_handler).ref_count.fetch_sub(1, Ordering::SeqCst) - 1 };

    if count == 0 {
        unsafe {
            Box::from_raw(render_process_handler);
        }
        1
    } else {
        0
    }
}

extern "C" fn has_one_ref(base: *mut cef_base_ref_counted_t) -> c_int {
    let render_process_handler = base as *mut RenderProcessHandler;
    let count = unsafe { (*render_process_handler).ref_count.load(Ordering::SeqCst) };
    if count == 1 {
        1
    } else {
        0
    }
}

extern "C" fn has_at_least_one_ref(base: *mut cef_base_ref_counted_t) -> c_int {
    let render_process_handler = base as *mut RenderProcessHandler;
    let count = unsafe { (*render_process_handler).ref_count.load(Ordering::SeqCst) };
    if count >= 1 {
        1
    } else {
        0
    }
}
