//! Safe-ish wrapper around libmpv configured to render a looping wallpaper into
//! an externally-owned OpenGL framebuffer via mpv's render API.
//!
//! Lifecycle:
//!   1. [`MpvPlayer::new`] creates the core, applies options and queues the
//!      media. Decoding can begin immediately.
//!   2. The backend makes an OpenGL context current, then calls
//!      [`MpvPlayer::init_render`] to attach the GL render context.
//!   3. Each frame the backend calls [`MpvPlayer::render`] with the target FBO.
//!   4. [`MpvPlayer::pump_events`] drains log/shutdown events.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::ptr;

use crate::config::{Fit, Resolved};
use crate::error::{Error, Result};
use crate::source::ResolvedSource;

use super::ffi;

/// Native display handle handed to mpv so hardware decoding can integrate with
/// the windowing system (improves vaapi/vdpau interop).
#[derive(Clone, Copy)]
pub enum NativeDisplay {
    /// Xlib `Display*`.
    X11(*mut c_void),
    /// `wl_display*`.
    Wayland(*mut c_void),
}

/// A single mpv instance driving one output's wallpaper.
pub struct MpvPlayer {
    handle: *mut ffi::mpv_handle,
    render: *mut ffi::mpv_render_context,
}

impl MpvPlayer {
    /// Create the mpv core, apply wallpaper-appropriate options, and queue the
    /// media for playback. Does **not** create the render context yet.
    pub fn new(settings: &Resolved, source: &ResolvedSource) -> Result<Self> {
        // SAFETY: mpv_create has no preconditions; returns null on failure.
        let handle = unsafe { ffi::mpv_create() };
        if handle.is_null() {
            return Err(Error::mpv("mpv_create() returned null"));
        }
        let player = MpvPlayer {
            handle,
            render: ptr::null_mut(),
        };

        player.apply_options(settings, source)?;

        // SAFETY: handle is valid and not yet initialised.
        check(unsafe { ffi::mpv_initialize(handle) }, "mpv_initialize")?;

        // Route mpv's diagnostics through our logger instead of its terminal.
        let level = CString::new("info").unwrap();
        unsafe {
            ffi::mpv_request_log_messages(handle, level.as_ptr());
        }

        player.load_source(source)?;
        Ok(player)
    }

    /// Apply all pre-initialisation options derived from the resolved settings.
    fn apply_options(&self, s: &Resolved, source: &ResolvedSource) -> Result<()> {
        // Use the embeddable render API video output.
        self.set_option("vo", "libmpv")?;
        self.set_option("hwdec", &s.hwdec)?;

        // Behave like a silent, headless, non-interactive background surface.
        for (k, v) in [
            ("terminal", "no"),
            ("config", "no"),
            ("osc", "no"),
            ("osd-level", "0"),
            ("load-scripts", "no"),
            ("ytdl", "no"),
            ("input-default-bindings", "no"),
            ("input-vo-keyboard", "no"),
            ("input-cursor", "no"),
            ("cursor-autohide", "no"),
            ("force-window", "no"),
            ("audio-display", "no"),
            // Keep the core alive when a file/playlist ends so the surface
            // never goes black.
            ("idle", "yes"),
            // Static images should stay up forever rather than "ending".
            ("image-display-duration", "inf"),
        ] {
            self.set_option(k, v)?;
        }

        // Audio.
        self.set_option("mute", if s.mute { "yes" } else { "no" })?;
        self.set_option("volume", &s.volume.to_string())?;

        // Looping.
        if s.loop_playback {
            self.set_option("loop-file", "inf")?;
            if source.is_playlist() {
                self.set_option("loop-playlist", "inf")?;
            }
        }

        // Scaling / fit.
        match s.fit {
            Fit::Cover => {
                self.set_option("keepaspect", "yes")?;
                self.set_option("panscan", "1.0")?;
            }
            Fit::Contain => {
                self.set_option("keepaspect", "yes")?;
                self.set_option("panscan", "0.0")?;
            }
            Fit::Fill => {
                self.set_option("keepaspect", "no")?;
            }
            Fit::Center => {
                self.set_option("keepaspect", "yes")?;
                self.set_option("video-unscaled", "yes")?;
            }
        }

        // Power-user passthrough: `--name=value` or bare `--flag`.
        for opt in &s.mpv_options {
            let trimmed = opt.trim_start_matches("--");
            let (name, value) = match trimmed.split_once('=') {
                Some((n, v)) => (n, v),
                None => (trimmed, "yes"),
            };
            if let Err(e) = self.set_option(name, value) {
                log::warn!("ignoring mpv option `{opt}`: {e}");
            }
        }
        Ok(())
    }

    /// (Re)load the resolved media (single file or playlist) for playback.
    ///
    /// Safe to call on a live player to swap the wallpaper without recreating
    /// the render context — the first entry replaces the current playlist.
    pub fn load_source(&self, source: &ResolvedSource) -> Result<()> {
        for (i, path) in source.files.iter().enumerate() {
            let path_c = CString::new(path.to_string_lossy().as_bytes())
                .map_err(|_| Error::mpv("media path contains an interior NUL byte"))?;
            // First entry replaces the current playlist; the rest append.
            let mode: &[u8] = if i == 0 { b"replace\0" } else { b"append\0" };
            let cmd = [
                b"loadfile\0".as_ptr() as *const c_char,
                path_c.as_ptr(),
                mode.as_ptr() as *const c_char,
                ptr::null(),
            ];
            // SAFETY: NUL-terminated argv of valid C strings, NULL terminated.
            check(
                unsafe { ffi::mpv_command(self.handle, cmd.as_ptr()) },
                "loadfile",
            )?;
        }
        Ok(())
    }

    /// Pause or resume playback (`pause` property).
    pub fn set_paused(&self, paused: bool) -> Result<()> {
        let value = if paused { "yes" } else { "no" };
        let name = CString::new("pause").unwrap();
        let value_c = CString::new(value).unwrap();
        // SAFETY: valid handle and NUL-terminated strings.
        let ret =
            unsafe { ffi::mpv_set_property_string(self.handle, name.as_ptr(), value_c.as_ptr()) };
        check(ret, "set pause")
    }

    /// Toggle the pause state without needing to read it first.
    pub fn toggle_paused(&self) -> Result<()> {
        let cmd = [
            b"cycle\0".as_ptr() as *const c_char,
            b"pause\0".as_ptr() as *const c_char,
            ptr::null(),
        ];
        // SAFETY: NUL-terminated argv of valid C strings, NULL terminated.
        check(
            unsafe { ffi::mpv_command(self.handle, cmd.as_ptr()) },
            "cycle pause",
        )
    }

    /// Mute or unmute audio (`mute` property).
    pub fn set_muted(&self, muted: bool) -> Result<()> {
        let value = if muted { "yes" } else { "no" };
        let name = CString::new("mute").unwrap();
        let value_c = CString::new(value).unwrap();
        // SAFETY: valid handle and NUL-terminated strings.
        let ret =
            unsafe { ffi::mpv_set_property_string(self.handle, name.as_ptr(), value_c.as_ptr()) };
        check(ret, "set mute")
    }

    /// Create the OpenGL render context. An OpenGL context **must** be current
    /// on the calling thread when this runs.
    ///
    /// `get_proc` resolves GL symbols; `update_cb` (if set) is invoked by mpv —
    /// possibly from another thread — when a new frame should be drawn.
    pub fn init_render(
        &mut self,
        display: NativeDisplay,
        get_proc: unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_void,
        update_cb: Option<(ffi::mpv_render_update_fn, *mut c_void)>,
    ) -> Result<()> {
        let mut init = ffi::mpv_opengl_init_params {
            get_proc_address: Some(get_proc),
            get_proc_address_ctx: ptr::null_mut(),
        };
        let mut advanced: std::os::raw::c_int = 0;

        // Build the parameter list. Per mpv's `render.h`, each param's `data`
        // holds a value of the documented "Type" directly: API_TYPE is a
        // `char*` (the string itself), the {X11,WL}_DISPLAY params are the
        // display pointer itself, while the struct/scalar params (FBO,
        // INIT_PARAMS, FLIP_Y, ADVANCED_CONTROL) are passed by address.
        let mut params: Vec<ffi::mpv_render_param> = vec![
            ffi::mpv_render_param {
                type_: ffi::MPV_RENDER_PARAM_API_TYPE,
                data: ffi::MPV_RENDER_API_TYPE_OPENGL.as_ptr() as *mut c_void,
            },
            ffi::mpv_render_param {
                type_: ffi::MPV_RENDER_PARAM_OPENGL_INIT_PARAMS,
                data: &mut init as *mut _ as *mut c_void,
            },
            ffi::mpv_render_param {
                type_: ffi::MPV_RENDER_PARAM_ADVANCED_CONTROL,
                data: &mut advanced as *mut _ as *mut c_void,
            },
        ];
        // The native-display entry lets mpv wire up hardware-decoding interop.
        match display {
            NativeDisplay::X11(disp) if !disp.is_null() => {
                params.push(ffi::mpv_render_param {
                    type_: ffi::MPV_RENDER_PARAM_X11_DISPLAY,
                    data: disp,
                });
            }
            NativeDisplay::Wayland(disp) if !disp.is_null() => {
                params.push(ffi::mpv_render_param {
                    type_: ffi::MPV_RENDER_PARAM_WL_DISPLAY,
                    data: disp,
                });
            }
            _ => {}
        }
        params.push(ffi::mpv_render_param {
            type_: ffi::MPV_RENDER_PARAM_INVALID,
            data: ptr::null_mut(),
        });

        let mut ctx: *mut ffi::mpv_render_context = ptr::null_mut();
        // SAFETY: params points to a valid, INVALID-terminated array; handle is
        // initialised; a GL context is current per this method's contract.
        check(
            unsafe { ffi::mpv_render_context_create(&mut ctx, self.handle, params.as_mut_ptr()) },
            "mpv_render_context_create",
        )?;
        self.render = ctx;

        if let Some((cb, cb_ctx)) = update_cb {
            // SAFETY: ctx is valid; cb/cb_ctx outlive the render context.
            unsafe {
                ffi::mpv_render_context_set_update_callback(ctx, Some(cb), cb_ctx);
            }
        }
        Ok(())
    }

    /// Render the current frame into the given framebuffer object.
    ///
    /// An OpenGL context must be current. `fbo` is the target framebuffer (0 =
    /// the window's default framebuffer).
    pub fn render(&self, fbo: i32, width: i32, height: i32) -> Result<()> {
        if self.render.is_null() {
            return Err(Error::mpv("render() called before init_render()"));
        }
        let mut fbo = ffi::mpv_opengl_fbo {
            fbo,
            w: width,
            h: height,
            internal_format: 0,
        };
        // Flip so mpv's bottom-left GL origin maps to the window's top-left.
        let mut flip_y: std::os::raw::c_int = 1;
        let mut params = [
            ffi::mpv_render_param {
                type_: ffi::MPV_RENDER_PARAM_OPENGL_FBO,
                data: &mut fbo as *mut _ as *mut c_void,
            },
            ffi::mpv_render_param {
                type_: ffi::MPV_RENDER_PARAM_FLIP_Y,
                data: &mut flip_y as *mut _ as *mut c_void,
            },
            ffi::mpv_render_param {
                type_: ffi::MPV_RENDER_PARAM_INVALID,
                data: ptr::null_mut(),
            },
        ];
        // SAFETY: render context valid; params INVALID-terminated; GL current.
        check(
            unsafe { ffi::mpv_render_context_render(self.render, params.as_mut_ptr()) },
            "mpv_render_context_render",
        )
    }

    /// Drain queued mpv events. Returns `true` if mpv asked to shut down.
    pub fn pump_events(&self) -> bool {
        loop {
            // SAFETY: handle valid; 0 timeout = non-blocking poll.
            let ev = unsafe { ffi::mpv_wait_event(self.handle, 0.0) };
            if ev.is_null() {
                return false;
            }
            // SAFETY: mpv guarantees a valid event pointer until the next call.
            let ev = unsafe { &*ev };
            match ev.event_id {
                ffi::MPV_EVENT_NONE => return false,
                ffi::MPV_EVENT_SHUTDOWN => return true,
                ffi::MPV_EVENT_LOG_MESSAGE => unsafe { log_message(ev.data) },
                ffi::MPV_EVENT_END_FILE => log::debug!("mpv: end-file"),
                ffi::MPV_EVENT_FILE_LOADED => log::debug!("mpv: file loaded"),
                _ => {}
            }
        }
    }

    fn set_option(&self, name: &str, value: &str) -> Result<()> {
        let name_c = CString::new(name).map_err(|_| Error::mpv("option name has NUL"))?;
        let value_c = CString::new(value).map_err(|_| Error::mpv("option value has NUL"))?;
        // SAFETY: both strings are valid NUL-terminated C strings.
        let ret =
            unsafe { ffi::mpv_set_option_string(self.handle, name_c.as_ptr(), value_c.as_ptr()) };
        check(ret, &format!("set_option({name}={value})"))
    }
}

impl Drop for MpvPlayer {
    fn drop(&mut self) {
        // The render context must be freed before the core is destroyed.
        if !self.render.is_null() {
            // SAFETY: render context is valid and owned by us.
            unsafe { ffi::mpv_render_context_free(self.render) };
            self.render = ptr::null_mut();
        }
        if !self.handle.is_null() {
            // SAFETY: handle is valid and owned by us.
            unsafe { ffi::mpv_terminate_destroy(self.handle) };
            self.handle = ptr::null_mut();
        }
    }
}

/// Translate an mpv error code into our error type, attaching the call name.
fn check(ret: std::os::raw::c_int, what: &str) -> Result<()> {
    if ret >= 0 {
        return Ok(());
    }
    // SAFETY: mpv_error_string always returns a valid static C string.
    let msg = unsafe { CStr::from_ptr(ffi::mpv_error_string(ret)) }
        .to_string_lossy()
        .into_owned();
    Err(Error::Mpv(format!("{what} failed: {msg} (code {ret})")))
}

/// Forward an mpv log message to our logger at a matching level.
///
/// # Safety
/// `data` must point to a valid `mpv_event_log_message` (as mpv guarantees for
/// `MPV_EVENT_LOG_MESSAGE`).
unsafe fn log_message(data: *mut c_void) {
    if data.is_null() {
        return;
    }
    let msg = unsafe { &*(data as *const ffi::mpv_event_log_message) };
    let prefix = unsafe { cstr(msg.prefix) };
    let level = unsafe { cstr(msg.level) };
    let text = unsafe { cstr(msg.text) };
    let text = text.trim_end();
    match level.as_str() {
        "fatal" | "error" => log::error!("mpv/{prefix}: {text}"),
        "warn" => log::warn!("mpv/{prefix}: {text}"),
        "info" => log::info!("mpv/{prefix}: {text}"),
        "v" | "debug" => log::debug!("mpv/{prefix}: {text}"),
        _ => log::trace!("mpv/{prefix}: {text}"),
    }
}

/// # Safety
/// `ptr` must be NULL or a valid NUL-terminated C string.
unsafe fn cstr(ptr: *const c_char) -> String {
    if ptr.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned()
    }
}
