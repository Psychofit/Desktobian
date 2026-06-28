//! EGL/OpenGL context and surface management, shared by the X11 and Wayland
//! backends.
//!
//! The `khronos-egl` "static" instance ([`egl::Static`]) is a zero-sized type,
//! so we can reconstruct it for free anywhere instead of threading it through
//! the program. The only state we actually need to carry around are the Copy
//! handles ([`egl::Display`], [`egl::Config`], [`egl::Surface`],
//! [`egl::Context`]).

use std::os::raw::{c_char, c_void};

use khronos_egl as egl;

use crate::error::{Error, Result};

/// Attributes for choosing a window-capable, OpenGL-renderable RGBA config.
const CONFIG_ATTRS: &[egl::Int] = &[
    egl::SURFACE_TYPE,
    egl::WINDOW_BIT,
    egl::RENDERABLE_TYPE,
    egl::OPENGL_BIT,
    egl::RED_SIZE,
    8,
    egl::GREEN_SIZE,
    8,
    egl::BLUE_SIZE,
    8,
    egl::ALPHA_SIZE,
    8,
    egl::NONE,
];

/// A request for a desktop OpenGL 3.0 context.
const CONTEXT_ATTRS: &[egl::Int] = &[
    egl::CONTEXT_MAJOR_VERSION,
    3,
    egl::CONTEXT_MINOR_VERSION,
    0,
    egl::NONE,
];

/// An initialised EGL display plus the chosen framebuffer config. One per
/// backend (i.e. per native display connection).
#[derive(Clone, Copy)]
pub struct EglDisplay {
    display: egl::Display,
    config: egl::Config,
}

impl EglDisplay {
    /// Initialise EGL on the given native display pointer (Xlib `Display*` or
    /// `wl_display*`).
    pub fn new(native_display: *mut c_void) -> Result<Self> {
        let egl = instance();
        // SAFETY: `native_display` is a live display pointer supplied by the
        // backend that owns it.
        let display = unsafe { egl.get_display(native_display) }
            .ok_or_else(|| Error::egl("eglGetDisplay returned no display"))?;

        let (major, minor) = egl
            .initialize(display)
            .map_err(|e| Error::egl(format!("eglInitialize failed: {e}")))?;
        log::debug!("EGL {major}.{minor} initialised");

        egl.bind_api(egl::OPENGL_API)
            .map_err(|e| Error::egl(format!("eglBindAPI(OpenGL) failed: {e}")))?;

        let config = egl
            .choose_first_config(display, CONFIG_ATTRS)
            .map_err(|e| Error::egl(format!("eglChooseConfig failed: {e}")))?
            .ok_or_else(|| Error::egl("no matching EGL config (need OpenGL + window + RGBA8)"))?;

        Ok(EglDisplay { display, config })
    }

    /// The X11 visual id matching the chosen config, used to create a
    /// compatible X window.
    pub fn native_visual_id(&self) -> Result<i32> {
        instance()
            .get_config_attrib(self.display, self.config, egl::NATIVE_VISUAL_ID)
            .map_err(|e| Error::egl(format!("querying NATIVE_VISUAL_ID failed: {e}")))
    }

    /// Create an OpenGL window surface + context bound to `native_window`.
    ///
    /// # Safety
    /// `native_window` must be a valid native window handle (an X11 `Window`
    /// XID cast to a pointer, or a `wl_egl_window*`) that outlives the returned
    /// [`GlSurface`].
    pub unsafe fn create_surface(&self, native_window: *mut c_void) -> Result<GlSurface> {
        let egl = instance();
        // SAFETY: caller guarantees `native_window` matches this display.
        let surface =
            unsafe { egl.create_window_surface(self.display, self.config, native_window, None) }
                .map_err(|e| Error::egl(format!("eglCreateWindowSurface failed: {e}")))?;

        let context = egl
            .create_context(self.display, self.config, None, CONTEXT_ATTRS)
            .map_err(|e| Error::egl(format!("eglCreateContext failed: {e}")))?;

        Ok(GlSurface {
            display: self.display,
            surface,
            context,
        })
    }
}

/// An EGL surface + context pair for one output. Rendering target for mpv.
pub struct GlSurface {
    display: egl::Display,
    surface: egl::Surface,
    context: egl::Context,
}

impl GlSurface {
    /// Make this surface/context current on the calling thread.
    pub fn make_current(&self) -> Result<()> {
        instance()
            .make_current(
                self.display,
                Some(self.surface),
                Some(self.surface),
                Some(self.context),
            )
            .map_err(|e| Error::egl(format!("eglMakeCurrent failed: {e}")))
    }

    /// Present the rendered frame.
    pub fn swap_buffers(&self) -> Result<()> {
        instance()
            .swap_buffers(self.display, self.surface)
            .map_err(|e| Error::egl(format!("eglSwapBuffers failed: {e}")))
    }

    /// Set the swap interval (1 = vsync, 0 = unthrottled). Best-effort.
    pub fn set_swap_interval(&self, interval: i32) {
        if let Err(e) = instance().swap_interval(self.display, interval) {
            log::debug!("eglSwapInterval({interval}) failed (non-fatal): {e}");
        }
    }
}

impl Drop for GlSurface {
    fn drop(&mut self) {
        let egl = instance();
        // Best-effort teardown; release current first to avoid destroying a
        // bound context/surface.
        let _ = egl.make_current(self.display, None, None, None);
        let _ = egl.destroy_context(self.display, self.context);
        let _ = egl.destroy_surface(self.display, self.surface);
    }
}

/// The mpv `get_proc_address` callback. Resolves GL symbols through EGL.
///
/// # Safety
/// Conforms to mpv's `get_proc_address` contract: `name` is a valid C string.
pub unsafe extern "C" fn mpv_get_proc_address(
    _ctx: *mut c_void,
    name: *const c_char,
) -> *mut c_void {
    if name.is_null() {
        return std::ptr::null_mut();
    }
    // SAFETY: mpv passes a valid NUL-terminated symbol name.
    let Ok(name) = (unsafe { std::ffi::CStr::from_ptr(name) }).to_str() else {
        return std::ptr::null_mut();
    };
    match instance().get_proc_address(name) {
        Some(f) => f as usize as *mut c_void,
        None => std::ptr::null_mut(),
    }
}

/// Construct the zero-sized static EGL instance.
fn instance() -> egl::Instance<egl::Static> {
    egl::Instance::new(egl::Static)
}
