use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::ptr;
use tokio::runtime::Runtime;

pub mod app;
pub mod audio;
pub mod config;
pub mod input;
pub mod speech;

use crate::app::TomChatApp;
use crate::config::Config;

/// C-compatible error codes
#[repr(C)]
pub enum TomChatResult {
    Success = 0,
    Error = 1,
    InvalidConfig = 2,
    AudioError = 3,
    TranscriptionError = 4,
}

/// Opaque handle for TomChat instance
pub struct TomChatHandle {
    runtime: Runtime,
    app: Option<TomChatApp>,
}

/// Initialize TomChat with config file path
/// Returns opaque handle or null on error
#[no_mangle]
pub extern "C" fn tomchat_init(config_path: *const c_char) -> *mut TomChatHandle {
    if config_path.is_null() {
        return ptr::null_mut();
    }

    let config_path_str = unsafe {
        match CStr::from_ptr(config_path).to_str() {
            Ok(s) => s,
            Err(_) => return ptr::null_mut(),
        }
    };

    let runtime = match Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return ptr::null_mut(),
    };

    let config = match runtime.block_on(async { Config::load() }) {
        Ok(cfg) => cfg,
        Err(_) => return ptr::null_mut(),
    };

    let app = match runtime.block_on(async { TomChatApp::new(config).await }) {
        Ok(app) => app,
        Err(_) => return ptr::null_mut(),
    };

    Box::into_raw(Box::new(TomChatHandle {
        runtime,
        app: Some(app),
    }))
}

/// Start TomChat background service
/// Note: This blocks the current thread
#[no_mangle]
pub extern "C" fn tomchat_start(handle: *mut TomChatHandle) -> c_int {
    if handle.is_null() {
        return TomChatResult::Error as c_int;
    }

    let handle_box = unsafe { &mut *handle };
    
    if let Some(app) = handle_box.app.take() {
        // Run app on this thread (caller should handle threading)
        match handle_box.runtime.block_on(async { app.run().await }) {
            Ok(_) => TomChatResult::Success as c_int,
            Err(_) => TomChatResult::Error as c_int,
        }
    } else {
        TomChatResult::Error as c_int
    }
}

/// Stop TomChat service
#[no_mangle]
pub extern "C" fn tomchat_stop(handle: *mut TomChatHandle) -> c_int {
    if handle.is_null() {
        return TomChatResult::Error as c_int;
    }

    // For now, stopping requires dropping the handle
    // In a more sophisticated implementation, we'd use channels for communication
    TomChatResult::Success as c_int
}

/// Get last error message
#[no_mangle]
pub extern "C" fn tomchat_get_last_error() -> *const c_char {
    // TODO: Implement proper error tracking
    CString::new("Not implemented").unwrap().into_raw()
}

/// Check if TomChat is running
#[no_mangle]
pub extern "C" fn tomchat_is_running(handle: *const TomChatHandle) -> c_int {
    if handle.is_null() {
        return 0;
    }
    // TODO: Implement proper status tracking
    1
}

/// Cleanup and free handle
#[no_mangle]
pub extern "C" fn tomchat_destroy(handle: *mut TomChatHandle) {
    if !handle.is_null() {
        unsafe {
            let _ = Box::from_raw(handle);
        }
    }
}

/// Set configuration parameter
#[no_mangle]
pub extern "C" fn tomchat_set_config(
    handle: *mut TomChatHandle,
    key: *const c_char,
    value: *const c_char,
) -> c_int {
    if handle.is_null() || key.is_null() || value.is_null() {
        return TomChatResult::InvalidConfig as c_int;
    }

    // TODO: Implement runtime config updates
    TomChatResult::Success as c_int
}