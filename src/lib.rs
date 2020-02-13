mod app;
mod bindings;
mod browser_process_handler;
mod client;
mod context_menu_handler;
mod display_handler;
mod life_span_handler;
mod request_handler;
mod render_process_handler;
mod schedule;
mod v8_pdf_print_handler;
mod v8_file_dialog_handler;
mod print_pdf_callback;
mod run_file_dialog_callback;

/// An actual browser within the CEF system
#[cfg(windows)]
#[path = "browser_windows.rs"]
mod browser;

pub use browser::Browser;

use std::mem::size_of;
use std::ptr::null_mut;
use std::sync::Arc;
use bindings::{
    cef_app_t, cef_execute_process, cef_initialize, cef_log_severity_t_LOGSEVERITY_ERROR,
    cef_log_severity_t_LOGSEVERITY_INFO, cef_main_args_t, cef_settings_t, cef_shutdown,
    cef_enable_highdpi_support,
};

/// The CEF system, including scheduler
pub struct Cef {
    schedule: Arc<schedule::Schedule>,
    _app: *mut app::App,
}

impl Cef {
    /// Initialize the CEF context and deal with forked processes. This should 
    /// generally be called as soon as possible in your application's lifetime
    #[cfg(windows)]
    pub fn initialize(
        debug_port: Option<u16>,
        enable_command_line_args: bool,
    ) -> Result<Cef, Box<dyn std::error::Error>> {
        // collect our args
        let main_args = unsafe {
            cef_main_args_t {
                instance: winapi::um::libloaderapi::GetModuleHandleA(null_mut())
                    as bindings::HINSTANCE,
            }
        };
    
        unsafe { cef_enable_highdpi_support() };
    
        log::debug!("preparing app");
        let schedule = Arc::new(schedule::Schedule::new());
        let app = app::allocate(schedule.clone());
    
        let exit_code = unsafe {
            (*app).inc_ref();
            cef_execute_process(&main_args, app as *mut cef_app_t, null_mut())
        };
        if exit_code >= 0 {
            std::process::exit(exit_code);
        }
    
        let mut settings = cef_settings_t::default();
        settings.size = size_of::<cef_settings_t>() as u64;
        settings.no_sandbox = 1;
        if let Some(port) = debug_port {
            settings.remote_debugging_port = port as i32;
        }
        settings.command_line_args_disabled = if enable_command_line_args { 0 } else { 1 };
        settings.multi_threaded_message_loop = 0;
        settings.external_message_pump = 1;
        if cfg!(debug_assertions) {
            settings.log_severity = cef_log_severity_t_LOGSEVERITY_INFO;
        } else {
            settings.log_severity = cef_log_severity_t_LOGSEVERITY_ERROR;
        }
    
        log::debug!("initializing");
        unsafe {
            (*app).inc_ref();
            if cef_initialize(&main_args, &settings, app as *mut cef_app_t, null_mut()) != 1 {
                return Err(Box::from("failed to initialize"));
            }
        }
    
        Ok(Cef {
            schedule,
            _app: app,
        })
    }
    
    /// Tell CEF to do its thing
    pub fn do_message_loop_work(&self) {
        unsafe { bindings::cef_do_message_loop_work(); }
    }

    /// Check whether or not the CEF scheduler is looking to process data
    pub fn should_do_work(&self) -> bool {
        self.schedule.should_do_work()
    }
}

impl Drop for Cef {
    fn drop(&mut self) {
        log::debug!("shutting down CEF..");
        unsafe { cef_shutdown() };
        log::debug!("CEF shutdown!");
    }
}
