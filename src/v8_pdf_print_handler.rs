use std::mem::size_of;
use std::os::raw::{c_int};
use std::sync::atomic::{AtomicUsize, Ordering};

use super::bindings::{
    cef_base_ref_counted_t, cef_v8handler_t, cef_string_t, cef_v8value_t, size_t,
    cef_string_userfree_t, cef_string_userfree_utf16_free, cef_frame_t, cef_v8context_t,
    cef_v8context_get_current_context
};

#[repr(C)]
pub struct V8PDFPrintHandler {
    v8_handler: cef_v8handler_t,
    ref_count: AtomicUsize,
    pub frame: Option<*mut cef_frame_t>,
    pub done_callback: Option<(*mut cef_v8context_t, *mut cef_v8value_t, *mut cef_v8value_t)>,
}

pub const CODE: &str = r#"
    var cef;
    if(!cef) cef = {};
    (function() {
        cef.printToPDF = function(path) {
            native function printToPDF(path, onDone, onError);
            return new Promise((resolve, reject) => {
                printToPDF(path, resolve, reject);
            });
        };
    })();
"#;

pub unsafe fn on_pdf_print_done(slf: *mut V8PDFPrintHandler, ok: bool) {
    if let Some((context, on_success, on_error)) = (*slf).done_callback {
        ((*context).enter.expect("enter is a function"))(context);

        // execute the appropriate callback
        if ok {
            ((*on_success).execute_function.expect("execute_function is a function"))(on_success, std::ptr::null_mut(), 0, std::ptr::null_mut());
        }
        else {
            ((*on_error).execute_function.expect("execute_function is a function"))(on_error, std::ptr::null_mut(), 0, std::ptr::null_mut());
        }

        ((*context).exit.expect("exit is a function"))(context);
        (*slf).done_callback = None;
    }
    else {
        log::warn!("pdf print is done but callback wasn't set?!");
    }
}

unsafe extern "C" fn execute(
    slf: *mut cef_v8handler_t,
    name: *const cef_string_t,
    _object: *mut cef_v8value_t,
    arguments_count: size_t,
    arguments: *const *mut cef_v8value_t,
    _retval: *mut *mut cef_v8value_t,
    _exception: *mut cef_string_t,
) -> c_int {
    // get the name of the function
    let chars: *mut u16 = (*name).str;
    let len: usize = (*name).length as usize;
    let chars = std::slice::from_raw_parts(chars, len);
    let name = std::char::decode_utf16(chars.iter().cloned())
        .map(|r| r.unwrap_or(std::char::REPLACEMENT_CHARACTER))
        .collect::<String>();

    if name == "printToPDF" && arguments_count == 3 {
        log::debug!("printing!");

        // get the path argument
        let arg0: *mut cef_v8value_t = *arguments;
        let is_string = ((*arg0).is_string.expect("is_string is a function"))(arg0) == 1;
        if !is_string {
            log::warn!("path argument isn't a string!");
            return 0;
        }

        // get the onDone argument
        let arg1: *mut cef_v8value_t = *(arguments.offset(1));
        let is_function = ((*arg1).is_function.expect("is_function is a function"))(arg1) == 1;
        if !is_function {
            log::warn!("onDone argument isn't a function!");
            return 0;
        }

        // get the onError argument
        let arg2: *mut cef_v8value_t = *(arguments.offset(2));
        let is_function = ((*arg2).is_function.expect("is_function is a function"))(arg2) == 1;
        if !is_function {
            log::warn!("onError argument isn't a function!");
            return 0;
        }

        // get the path as a string
        let cef_path: cef_string_userfree_t = ((*arg0).get_string_value.expect("get_string_value is a function"))(arg0);
        let chars: *mut u16 = (*cef_path).str;
        let len: usize = (*cef_path).length as usize;
        let chars = std::slice::from_raw_parts(chars, len);
        let path = std::char::decode_utf16(chars.iter().cloned())
            .map(|r| r.unwrap_or(std::char::REPLACEMENT_CHARACTER))
            .collect::<String>();
        log::debug!("printing PDF to path `{}`...", path);

        // now send an IPC message to the frame process telling it to print
        let _self = slf as *mut V8PDFPrintHandler;
        if let Some(frame) = (*_self).frame {
            // convert the message name to a CEF string
            let mut cef_message_name = cef_string_t::default();
            let message_name = "print_to_pdf".as_bytes();
            let message_name = std::ffi::CString::new(message_name).unwrap();
            super::bindings::cef_string_utf8_to_utf16(message_name.as_ptr(), message_name.to_bytes().len() as u64, &mut cef_message_name);

            // store our callback to onDone
            let context = cef_v8context_get_current_context();
            (*_self).done_callback = Some((context, arg1, arg2));

            // build the message
            let message = super::bindings::cef_process_message_create(&cef_message_name);
            let args = ((*message).get_argument_list.expect("get_argument_list is a function"))(message);
            ((*args).set_size.expect("set_size is a function"))(args, 1);
            ((*args).set_string.expect("set_string is a function"))(args, 0, cef_path);

            // send the message
            ((*frame).send_process_message.expect("send_process_message is a function"))(frame, super::bindings::cef_process_id_t_PID_BROWSER, message);
        }
        else {
            log::error!("frame isn't set!");
        }

        cef_string_userfree_utf16_free(cef_path);
        1
    }
    else {
        log::warn!("unrecognized function: `{}` with {} args, skipping", name, arguments_count);
        0
    }
}

pub fn allocate() -> *mut V8PDFPrintHandler {
    let handler = V8PDFPrintHandler {
        v8_handler: cef_v8handler_t {
            base: cef_base_ref_counted_t {
                size: size_of::<V8PDFPrintHandler>() as u64,
                add_ref: Some(add_ref),
                release: Some(release),
                has_one_ref: Some(has_one_ref),
                has_at_least_one_ref: Some(has_at_least_one_ref),
            },
            execute: Some(execute),
        },
        ref_count: AtomicUsize::new(1),
        frame: None,
        done_callback: None,
    };

    Box::into_raw(Box::from(handler))
}

extern "C" fn add_ref(base: *mut cef_base_ref_counted_t) {
    let v8_handler = base as *mut V8PDFPrintHandler;
    unsafe { (*v8_handler).ref_count.fetch_add(1, Ordering::SeqCst) };
}

extern "C" fn release(base: *mut cef_base_ref_counted_t) -> c_int {
    let v8_handler = base as *mut V8PDFPrintHandler;
    let count = unsafe { (*v8_handler).ref_count.fetch_sub(1, Ordering::SeqCst) - 1 };

    if count == 0 {
        unsafe {
            Box::from_raw(v8_handler);
        }
        1
    } else {
        0
    }
}

extern "C" fn has_one_ref(base: *mut cef_base_ref_counted_t) -> c_int {
    let v8_handler = base as *mut V8PDFPrintHandler;
    let count = unsafe { (*v8_handler).ref_count.load(Ordering::SeqCst) };
    if count == 1 {
        1
    } else {
        0
    }
}

extern "C" fn has_at_least_one_ref(base: *mut cef_base_ref_counted_t) -> c_int {
    let v8_handler = base as *mut V8PDFPrintHandler;
    let count = unsafe { (*v8_handler).ref_count.load(Ordering::SeqCst) };
    if count >= 1 {
        1
    } else {
        0
    }
}
