use super::bindings::{
    cef_window_info_t, cef_string_t, cef_string_utf8_to_utf16, cef_browser_settings_t,
    cef_state_t_STATE_DISABLED, cef_dictionary_value_create, cef_request_context_get_global_context,
    cef_browser_host_create_browser_sync, cef_browser_host_t, cef_browser_t,
};
use winapi::shared::windef::HWND;
use std::ffi::CString;
use std::ptr::null_mut;

/// The browser, keeping track of everything including its host
pub struct Browser {
    _browser: *mut cef_browser_t,
    client: *mut super::client::Client,
    host: *mut cef_browser_host_t,
    pub hwnd: HWND,
}

impl Browser {
    /// Create the browser as a child of a standard windows HWND
    pub fn create(window_name: &str, parent_window: HWND, url: &str, width: i32, height: i32) -> Browser {
        let mut cef_window_name = cef_string_t::default();
        let window_name = window_name.as_bytes();
        let window_name = CString::new(window_name).unwrap();
        unsafe { cef_string_utf8_to_utf16(window_name.as_ptr(), window_name.to_bytes().len() as u64, &mut cef_window_name); }
        use winapi::um::winuser::{WS_CHILD, WS_CLIPCHILDREN, WS_CLIPSIBLINGS, WS_TABSTOP, WS_VISIBLE};
        let window_info = cef_window_info_t {
            ex_style: 0,
            window_name: cef_window_name,
            style: WS_CHILD | WS_CLIPCHILDREN | WS_CLIPSIBLINGS | WS_TABSTOP | WS_VISIBLE,
            x: 0,
            y: 0,
            width,
            height,
            parent_window: parent_window as super::bindings::HWND,
            menu: null_mut(),
            windowless_rendering_enabled: 0,
            shared_texture_enabled: 0,
            external_begin_frame_enabled: 0,
            window: null_mut(),
        };
    
        let client = super::client::allocate();
    
        let mut cef_url = cef_string_t::default();
        let url = url.as_bytes();
        let url = CString::new(url).unwrap();
        unsafe { cef_string_utf8_to_utf16(url.as_ptr(), url.to_bytes().len() as u64, &mut cef_url); }
    
        let mut browser_settings = cef_browser_settings_t::default();
        browser_settings.databases = cef_state_t_STATE_DISABLED;
        browser_settings.local_storage = cef_state_t_STATE_DISABLED;
        browser_settings.application_cache = cef_state_t_STATE_DISABLED;
        
        let browser = unsafe {
            cef_browser_host_create_browser_sync(
                &window_info,
                client as *mut super::bindings::cef_client_t,
                &cef_url,
                &browser_settings,
                cef_dictionary_value_create(),
                cef_request_context_get_global_context()
            )
        };
    
        let host = unsafe {
            (*browser).get_host.unwrap()(browser)
        };
        let hwnd = unsafe {
            (*host).get_window_handle.unwrap()(host)
        };
    
        Browser {
            _browser: browser,
            client,
            host,
            hwnd: hwnd as HWND,
        }
    }

    pub fn set_fullscreen_listener<F: FnMut(bool) + 'static>(&self, listener: F) {
        unsafe { super::client::set_fullscreen_listener(self.client, listener); }
    }

    /// Resize the browser window, call this whenever the host resizes
    pub fn resize(&self, width: i32, height: i32) {
        use winapi::um::winuser::{SetWindowPos, SWP_NOZORDER};

        unsafe {
            (*self.host).notify_move_or_resize_started.unwrap()(self.host);
            SetWindowPos(self.hwnd, null_mut(), 0, 0, width, height, SWP_NOZORDER);
            (*self.host).was_resized.unwrap()(self.host);
        }
    }

    /// Close the browser instance
    pub fn try_close(&self) -> bool {
        let closed = unsafe { (*self.host).try_close_browser.unwrap()(self.host) };
        closed == 1
    }
}