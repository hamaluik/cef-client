#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{mem, ptr};
use winapi::shared::minwindef::{LPARAM, UINT, WPARAM, LRESULT, HINSTANCE};
use winapi::shared::windef::HWND;

static mut H_INSTANCE: HINSTANCE = ptr::null_mut();

struct WindowData {
    browser: cef_client::Browser,
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    use winapi::um::winuser::{WM_SIZE, WM_ERASEBKGND, WM_CLOSE, WM_DESTROY, DestroyWindow, PostQuitMessage, DefWindowProcW, GetWindowLongPtrW, GetClientRect };

    // we store a pointer to our data as extra data on the window
    // extract it here
    let ptr = GetWindowLongPtrW(hwnd, 0) as *mut u8;
    let data_ptr = ptr as *mut *mut WindowData;
    // **only** dereference the pointer to a pointer if the outer pointer isn't 0
    // otherwise we get heap corruption
    let data: Option<&mut WindowData> = if data_ptr as isize != 0 {
        Some(&mut *(*data_ptr))
    } else {
        None
    };

    match msg {
        WM_SIZE => {
            if data.is_none() {
                return DefWindowProcW(hwnd, msg, wparam, lparam);
            }

            let mut rect = mem::MaybeUninit::uninit();
            GetClientRect(hwnd, rect.as_mut_ptr());
            let rect = rect.assume_init();

            data.unwrap().browser.resize(rect.right - rect.left, rect.bottom - rect.top);
            0
        },

        // don't erase the background if the browser has been loaded to prevent flashing
        WM_ERASEBKGND => if data.is_some() { 0 } else { DefWindowProcW(hwnd, msg, wparam, lparam) },

        WM_CLOSE => {
            if data.is_none() {
                log::warn!("closing window before browser was created!");
                DestroyWindow(hwnd);
                0
            }
            else {
                log::debug!("trying to close browser window...");
                if data.unwrap().browser.try_close() {
                    log::debug!("browser closed, destroying window...");
                    DestroyWindow(hwnd);
                    0
                }
                else {
                    log::warn!("failed to close browser window, not destroying parent!");
                    0
                }
            }
        },

        WM_DESTROY => {
            log::debug!("posting quit message(0)...");
            PostQuitMessage(0);
            0
        },

        _ => DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        H_INSTANCE = winapi::um::libloaderapi::GetModuleHandleW(ptr::null_mut());
    }

    // setup logging
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}][{}] {}",
                record.target(),
                record.level(),
                message
            ))
        })
        .chain(std::io::stdout())
        .apply()?;

    // initialize CEF
    let mut cef = cef_client::Cef::initialize()?;

    // load our icon
    use winapi::um::winuser::{MAKEINTRESOURCEW, LoadImageW, IMAGE_ICON, LR_DEFAULTSIZE};
    use winapi::um::winnt::HANDLE;
    use winapi::shared::windef::HICON;
    let icon: HANDLE = unsafe {
        LoadImageW(H_INSTANCE, MAKEINTRESOURCEW(1), IMAGE_ICON, 0, 0, LR_DEFAULTSIZE)
    };

    // create our window class
    use winapi::um::winuser::{WNDCLASSW, CS_HREDRAW, CS_VREDRAW, LoadCursorW, IDC_ARROW, RegisterClassW, CreateWindowExW, WS_OVERLAPPEDWINDOW, ShowWindow, SW_SHOW, GetMessageW, TranslateMessage, DispatchMessageW, SetWindowLongPtrW, GetDesktopWindow, GetWindowRect, SetWindowPos, HWND_TOP, GetClientRect };
    use winapi::shared::windef::HBRUSH;
    let class_name: Vec<u16> = "cef-win-print\0".encode_utf16().collect();
    let wnd_class = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(wndproc),
        hInstance: unsafe { H_INSTANCE },
        lpszClassName: class_name.as_ptr(),
        cbClsExtra: 0,
        cbWndExtra: std::mem::size_of::<*mut WindowData>() as i32,
        hIcon: icon as HICON,
        hCursor: unsafe { LoadCursorW(H_INSTANCE, IDC_ARROW) },
        hbrBackground: winapi::um::winuser::COLOR_WINDOW as HBRUSH,
        lpszMenuName: ptr::null_mut(),
    };
    unsafe { RegisterClassW(&wnd_class) };
    
    // create the window
    let window_title: Vec<u16> = "CEF Printing Demo\0".encode_utf16().collect();
    let hwnd = unsafe {
        CreateWindowExW(
            0,
            class_name.as_ptr(),
            window_title.as_ptr(),
            WS_OVERLAPPEDWINDOW,
            0,
            0,
            1280,
            720,
            ptr::null_mut(),
            ptr::null_mut(),
            H_INSTANCE,
            ptr::null_mut(),
        )
    };

    // size and center the window
    let mut rect = mem::MaybeUninit::uninit();
    unsafe { GetWindowRect(hwnd, rect.as_mut_ptr()); }
    let mut rect = unsafe{ rect.assume_init() };
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    unsafe { GetWindowRect(GetDesktopWindow(), &mut rect); }
    unsafe { SetWindowPos(hwnd, HWND_TOP, (rect.right - width) / 2, (rect.bottom - height) / 2, width, height, 0); }

    // now show the window!
    unsafe { ShowWindow(hwnd, SW_SHOW); }

    // create the browser
    unsafe { GetClientRect(hwnd, &mut rect); }
    use urlencoding::encode;
    let browser = cef.create_browser("my_cef_window", hwnd, &format!("data:text/html,{}", encode(include_str!("page.html"))), rect.right - rect.left, rect.bottom - rect.top);

    // and give the window our data struct
    let mut data: WindowData = WindowData {
        browser,
    };
    unsafe {
        let data_ptr: *mut WindowData = &mut data;
        SetWindowLongPtrW(
            hwnd,
            0,
            &data_ptr as *const *mut WindowData as isize,
        );
    }

    // finally, the message loop
    unsafe {
        let mut msg = mem::MaybeUninit::uninit();
        'mainloop: loop {
            let ret = GetMessageW(msg.as_mut_ptr(), ptr::null_mut(), 0, 0);
            if ret == -1 {
                log::error!("message error!");
                return Err(Box::from("TODO: message error"));
            }
            else if ret == 0 {
                log::debug!("time to quit!");
                break 'mainloop;
            }
            else {
                let msg = msg.assume_init();
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            
            if cef.should_do_work() {
                cef.do_message_loop_work();
            }
        }
    }

    log::info!("shutting down...");

    Ok(())
}
