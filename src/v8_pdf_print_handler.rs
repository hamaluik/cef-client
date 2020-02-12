use std::mem::size_of;
use std::os::raw::{c_int};
use std::sync::atomic::{AtomicUsize, Ordering};

use super::bindings::{
    cef_base_ref_counted_t, cef_v8handler_t, cef_string_t, cef_v8value_t, size_t,
    cef_string_userfree_t, cef_string_userfree_utf16_free, cef_browser_host_t, cef_string_utf8_to_utf16
};
use super::print_pdf_callback;

#[repr(C)]
pub struct V8PDFPrintHandler {
    v8_handler: cef_v8handler_t,
    ref_count: AtomicUsize,
    pub host: Option<*mut cef_browser_host_t>,
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

    if name == "printToPDF" && arguments_count == 1 {
        // get the path argument
        log::debug!("printing!");
        let arg0: *mut cef_v8value_t = *arguments;
        let is_string = ((*arg0).is_string.expect("is_string is a function"))(arg0) == 1;
        if !is_string {
            log::warn!("path argument isn't a string!");
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
        cef_string_userfree_utf16_free(cef_path);

        // now get the browser to print!
        let _self = slf as *mut V8PDFPrintHandler;
        if let Some(host) = (*_self).host {
            log::debug!("got host: {:p}", host);
            let host = &mut (*host);
            // and start printing
            if let Some(print) = host.print_to_pdf {
                // first, convert the path to a cef string
                log::debug!("converting path string to cef string");
                use std::ffi::CString;
                let path = path.as_bytes();
                let path = CString::new(path).unwrap();
                let mut cef_path = cef_string_t::default();
                cef_string_utf8_to_utf16(path.as_ptr(), path.to_bytes().len() as u64, &mut cef_path);

                // determine the settings
                // note: page size in microns, to get microns from inches, multiply
                // by 25400.
                log::debug!("creating settings");
                let settings = super::bindings::_cef_pdf_print_settings_t {
                    header_footer_title: cef_string_t::default(), // empty header / footer
                    header_footer_url: cef_string_t::default(), // empty url
                    page_width: 215900, // 8.5 inches (letterpaper)
                    page_height: 279400, // 11 inches (letterpaper)
                    scale_factor: 100, // scale the page at 100%
                    margin_top: 25.4, // margins in millimeters (actually ignored becayse of margin type)
                    margin_right: 25.4,
                    margin_bottom: 25.4,
                    margin_left: 25.4,
                    margin_type: super::bindings::cef_pdf_print_margin_type_t_PDF_PRINT_MARGIN_DEFAULT, // default margins
                    header_footer_enabled: 0, // no headers or footers
                    selection_only: 0, // print everything
                    landscape: 0, // portrait mode
                    backgrounds_enabled: 1, // show background colours / graphics
                };

                // now a callback when the print is done
                log::debug!("allocating done callback");
                let callback = print_pdf_callback::allocate();

                // finally, initiate the print
                log::debug!("initiating printing...");
                print(host, &mut cef_path, &settings, callback as *mut super::bindings::_cef_pdf_print_callback_t);
            }
            else {
                log::warn!("print_to_pdf callback is null! NOT printing!");
            }
        }
        else {
            log::error!("browser isn't set!")
        }

        1
    }
    else {
        log::debug!("unrecognized function: `{}` with {} args, skipping", name, arguments_count);
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
        host: None,
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
