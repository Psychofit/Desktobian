//! Hand-written FFI bindings for the subset of libmpv that Desktobian uses.
//!
//! These are transcribed directly from the installed `<mpv/client.h>`,
//! `<mpv/render.h>` and `<mpv/render_gl.h>` headers (libmpv ABI, stable since
//! mpv 0.36). We bind by hand rather than pulling in `bindgen`/`libclang` so the
//! build stays lightweight and reproducible.
//!
//! Everything here is `unsafe` by nature; the safe surface lives in
//! [`super::mpv`].
//!
//! This is a binding module: it intentionally declares the full set of
//! constants/functions we transcribed, even those not yet called.
#![allow(non_camel_case_types)]
#![allow(dead_code)]

use std::os::raw::{c_char, c_int, c_void};

/// Opaque mpv client handle (`mpv_handle`).
#[repr(C)]
pub struct mpv_handle {
    _private: [u8; 0],
}

/// Opaque render context (`mpv_render_context`).
#[repr(C)]
pub struct mpv_render_context {
    _private: [u8; 0],
}

// --- mpv_format (client.h) -------------------------------------------------
pub const MPV_FORMAT_NONE: c_int = 0;
pub const MPV_FORMAT_STRING: c_int = 1;
pub const MPV_FORMAT_FLAG: c_int = 3;
pub const MPV_FORMAT_INT64: c_int = 4;
pub const MPV_FORMAT_DOUBLE: c_int = 5;

// --- mpv_event_id (client.h) ----------------------------------------------
pub const MPV_EVENT_NONE: c_int = 0;
pub const MPV_EVENT_SHUTDOWN: c_int = 1;
pub const MPV_EVENT_LOG_MESSAGE: c_int = 2;
pub const MPV_EVENT_END_FILE: c_int = 7;
pub const MPV_EVENT_FILE_LOADED: c_int = 8;

/// `mpv_event` — fixed layout: { i32, i32, u64, ptr } = 24 bytes on LP64.
#[repr(C)]
pub struct mpv_event {
    pub event_id: c_int,
    pub error: c_int,
    pub reply_userdata: u64,
    pub data: *mut c_void,
}

/// `mpv_event_log_message`.
#[repr(C)]
pub struct mpv_event_log_message {
    pub prefix: *const c_char,
    pub level: *const c_char,
    pub text: *const c_char,
    pub log_level: c_int,
}

// --- render.h param types --------------------------------------------------
pub const MPV_RENDER_PARAM_INVALID: c_int = 0;
pub const MPV_RENDER_PARAM_API_TYPE: c_int = 1;
pub const MPV_RENDER_PARAM_OPENGL_INIT_PARAMS: c_int = 2;
pub const MPV_RENDER_PARAM_OPENGL_FBO: c_int = 3;
pub const MPV_RENDER_PARAM_FLIP_Y: c_int = 4;
pub const MPV_RENDER_PARAM_X11_DISPLAY: c_int = 8;
pub const MPV_RENDER_PARAM_WL_DISPLAY: c_int = 9;
pub const MPV_RENDER_PARAM_ADVANCED_CONTROL: c_int = 10;

/// `MPV_RENDER_API_TYPE_OPENGL` is the C string "opengl".
pub const MPV_RENDER_API_TYPE_OPENGL: &[u8] = b"opengl\0";

/// Bit returned by `mpv_render_context_update` meaning "a new frame is ready".
pub const MPV_RENDER_UPDATE_FRAME: u64 = 1 << 0;

/// `mpv_render_param { enum type; void *data; }`.
#[repr(C)]
pub struct mpv_render_param {
    pub type_: c_int,
    pub data: *mut c_void,
}

/// `mpv_opengl_init_params`.
#[repr(C)]
pub struct mpv_opengl_init_params {
    pub get_proc_address:
        Option<unsafe extern "C" fn(ctx: *mut c_void, name: *const c_char) -> *mut c_void>,
    pub get_proc_address_ctx: *mut c_void,
}

/// `mpv_opengl_fbo`.
#[repr(C)]
pub struct mpv_opengl_fbo {
    pub fbo: c_int,
    pub w: c_int,
    pub h: c_int,
    pub internal_format: c_int,
}

/// Update callback signature shared by the render context.
pub type mpv_render_update_fn = unsafe extern "C" fn(cb_ctx: *mut c_void);
/// Wakeup callback signature for the core event loop.
pub type mpv_wakeup_fn = unsafe extern "C" fn(d: *mut c_void);

extern "C" {
    pub fn mpv_create() -> *mut mpv_handle;
    pub fn mpv_initialize(ctx: *mut mpv_handle) -> c_int;
    pub fn mpv_terminate_destroy(ctx: *mut mpv_handle);
    pub fn mpv_error_string(error: c_int) -> *const c_char;

    pub fn mpv_set_option_string(
        ctx: *mut mpv_handle,
        name: *const c_char,
        data: *const c_char,
    ) -> c_int;
    pub fn mpv_set_property_string(
        ctx: *mut mpv_handle,
        name: *const c_char,
        data: *const c_char,
    ) -> c_int;
    pub fn mpv_set_property(
        ctx: *mut mpv_handle,
        name: *const c_char,
        format: c_int,
        data: *mut c_void,
    ) -> c_int;
    pub fn mpv_command(ctx: *mut mpv_handle, args: *const *const c_char) -> c_int;
    pub fn mpv_command_string(ctx: *mut mpv_handle, args: *const c_char) -> c_int;

    pub fn mpv_request_log_messages(ctx: *mut mpv_handle, min_level: *const c_char) -> c_int;
    pub fn mpv_wait_event(ctx: *mut mpv_handle, timeout: f64) -> *mut mpv_event;
    pub fn mpv_set_wakeup_callback(ctx: *mut mpv_handle, cb: Option<mpv_wakeup_fn>, d: *mut c_void);

    pub fn mpv_render_context_create(
        res: *mut *mut mpv_render_context,
        mpv: *mut mpv_handle,
        params: *mut mpv_render_param,
    ) -> c_int;
    pub fn mpv_render_context_set_update_callback(
        ctx: *mut mpv_render_context,
        callback: Option<mpv_render_update_fn>,
        callback_ctx: *mut c_void,
    );
    pub fn mpv_render_context_update(ctx: *mut mpv_render_context) -> u64;
    pub fn mpv_render_context_render(
        ctx: *mut mpv_render_context,
        params: *mut mpv_render_param,
    ) -> c_int;
    pub fn mpv_render_context_free(ctx: *mut mpv_render_context);
}
