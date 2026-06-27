//! X11 backend.
//!
//! For each monitor (enumerated via XRandR) we create a borderless,
//! override-redirect window covering that monitor's geometry, tag it as a
//! desktop window, lower it to the bottom of the stack, and render the
//! wallpaper into it with an EGL/OpenGL surface driven by mpv.
//!
//! This is the same approach used by tools like `xwinwrap`: it works great on
//! minimal window managers (i3, bspwm, openbox, …) and on X11 sessions of the
//! big desktops. (Full GNOME/KDE desktop-layer integration is tracked in the
//! roadmap.)

use std::os::raw::{c_int, c_long, c_uchar, c_uint, c_ulong};
use std::ptr;
use std::time::{Duration, Instant};

use x11_dl::xlib::{self, Xlib};
use x11_dl::xrandr::Xrandr;

use crate::backend::{Backend, WallpaperPlan};
use crate::error::{Error, Result};
use crate::monitor::Output;
use crate::player::{MpvPlayer, NativeDisplay};
use crate::render::{mpv_get_proc_address, EglDisplay, GlSurface};
use crate::util;

// --- X11 ABI constants (from X.h / Xrandr); not exported by x11-dl. ----------
const ALLOC_NONE: c_int = 0;
const INPUT_OUTPUT: c_uint = 1;
const CW_BACK_PIXEL: c_ulong = 1 << 1;
const CW_BORDER_PIXEL: c_ulong = 1 << 3;
const CW_OVERRIDE_REDIRECT: c_ulong = 1 << 9;
const CW_EVENT_MASK: c_ulong = 1 << 11;
const CW_COLORMAP: c_ulong = 1 << 13;
const EXPOSURE_MASK: c_long = 1 << 15;
const STRUCTURE_NOTIFY_MASK: c_long = 1 << 17;
const VISUAL_ID_MASK: c_long = 0x1;
const PROP_MODE_REPLACE: c_int = 0;
const XA_ATOM: c_ulong = 4;
const TRUE: i32 = 1;

/// The X11 backend holds the Xlib connection plus the initialised EGL display.
pub struct X11Backend {
    xlib: Xlib,
    xrandr: Xrandr,
    display: *mut xlib::Display,
    root: xlib::Window,
    egl: EglDisplay,
}

impl X11Backend {
    /// Open the X display and initialise EGL on it.
    pub fn connect() -> Result<Self> {
        let xlib = Xlib::open().map_err(|e| Error::X11(format!("cannot load Xlib: {e}")))?;
        let xrandr = Xrandr::open().map_err(|e| Error::X11(format!("cannot load XRandR: {e}")))?;

        // SAFETY: passing NULL uses $DISPLAY; returns NULL on failure.
        let display = unsafe { (xlib.XOpenDisplay)(ptr::null()) };
        if display.is_null() {
            return Err(Error::X11(
                "cannot open X display (is $DISPLAY set?)".into(),
            ));
        }
        // SAFETY: display is valid.
        let root = unsafe { (xlib.XDefaultRootWindow)(display) };

        let egl = EglDisplay::new(display as *mut _)?;

        Ok(X11Backend {
            xlib,
            xrandr,
            display,
            root,
            egl,
        })
    }

    /// Create the desktop-layer window for one monitor and an EGL surface on it.
    fn create_window(&self, output: &Output) -> Result<(xlib::Window, GlSurface)> {
        let visual_id = self.egl.native_visual_id()? as xlib::VisualID;

        // Find the X visual that matches the EGL config.
        let mut template: xlib::XVisualInfo = unsafe { std::mem::zeroed() };
        template.visualid = visual_id;
        let mut count: c_int = 0;
        // SAFETY: template/count are valid; returns an XFree-owned array.
        let vinfo = unsafe {
            (self.xlib.XGetVisualInfo)(self.display, VISUAL_ID_MASK, &mut template, &mut count)
        };
        if vinfo.is_null() || count == 0 {
            return Err(Error::X11(format!(
                "no X visual matches EGL visual id {visual_id}"
            )));
        }
        // SAFETY: vinfo points to at least one XVisualInfo.
        let vinfo_ref = unsafe { &*vinfo };
        let depth = vinfo_ref.depth;
        let visual = vinfo_ref.visual;

        // SAFETY: root/visual are valid; AllocNone needs no entries.
        let colormap =
            unsafe { (self.xlib.XCreateColormap)(self.display, self.root, visual, ALLOC_NONE) };

        let mut attrs: xlib::XSetWindowAttributes = unsafe { std::mem::zeroed() };
        attrs.background_pixel = 0;
        attrs.border_pixel = 0;
        attrs.colormap = colormap;
        attrs.override_redirect = TRUE;
        attrs.event_mask = EXPOSURE_MASK | STRUCTURE_NOTIFY_MASK;
        let valuemask =
            CW_BACK_PIXEL | CW_BORDER_PIXEL | CW_COLORMAP | CW_OVERRIDE_REDIRECT | CW_EVENT_MASK;

        // SAFETY: all handles valid; attrs initialised for the given valuemask.
        let window = unsafe {
            (self.xlib.XCreateWindow)(
                self.display,
                self.root,
                output.x,
                output.y,
                output.width as c_uint,
                output.height as c_uint,
                0,
                depth,
                INPUT_OUTPUT,
                visual,
                valuemask,
                &mut attrs,
            )
        };
        // SAFETY: vinfo was allocated by Xlib and is no longer needed.
        unsafe { (self.xlib.XFree)(vinfo as *mut _) };

        if window == 0 {
            return Err(Error::X11("XCreateWindow failed".into()));
        }

        self.mark_desktop_window(window);
        self.set_window_name(window, "desktobian");

        // SAFETY: window is valid.
        unsafe {
            (self.xlib.XLowerWindow)(self.display, window);
            (self.xlib.XMapWindow)(self.display, window);
            (self.xlib.XLowerWindow)(self.display, window);
            (self.xlib.XSync)(self.display, 0);
        }

        // SAFETY: `window` is a live X window of a visual compatible with the
        // EGL config; it outlives the surface (dropped together by the caller).
        let surface = unsafe { self.egl.create_surface(window as *mut _) }?;
        Ok((window, surface))
    }

    /// Tag the window with `_NET_WM_WINDOW_TYPE = _NET_WM_WINDOW_TYPE_DESKTOP`.
    fn mark_desktop_window(&self, window: xlib::Window) {
        let wm_type = self.intern("_NET_WM_WINDOW_TYPE");
        let desktop = self.intern("_NET_WM_WINDOW_TYPE_DESKTOP");
        if wm_type == 0 || desktop == 0 {
            return;
        }
        let value = desktop;
        // SAFETY: valid display/window/atoms; one 32-bit ATOM element.
        unsafe {
            (self.xlib.XChangeProperty)(
                self.display,
                window,
                wm_type,
                XA_ATOM,
                32,
                PROP_MODE_REPLACE,
                &value as *const c_ulong as *const c_uchar,
                1,
            );
        }
    }

    fn set_window_name(&self, window: xlib::Window, name: &str) {
        if let Ok(c) = std::ffi::CString::new(name) {
            // SAFETY: valid display/window; c is a valid C string.
            unsafe { (self.xlib.XStoreName)(self.display, window, c.as_ptr()) };
        }
    }

    fn intern(&self, name: &str) -> xlib::Atom {
        let Ok(c) = std::ffi::CString::new(name) else {
            return 0;
        };
        // SAFETY: display valid; c is a valid C string; only-if-exists = false.
        unsafe { (self.xlib.XInternAtom)(self.display, c.as_ptr(), 0) }
    }

    /// Read the human-readable name of an XRandR monitor (an interned atom).
    fn atom_name(&self, atom: xlib::Atom) -> Option<String> {
        if atom == 0 {
            return None;
        }
        // SAFETY: display/atom valid; returns an XFree-owned C string or NULL.
        let raw = unsafe { (self.xlib.XGetAtomName)(self.display, atom) };
        if raw.is_null() {
            return None;
        }
        // SAFETY: raw is a valid C string owned by Xlib.
        let name = unsafe { std::ffi::CStr::from_ptr(raw) }
            .to_string_lossy()
            .into_owned();
        // SAFETY: raw was allocated by Xlib.
        unsafe { (self.xlib.XFree)(raw as *mut _) };
        Some(name)
    }
}

impl Backend for X11Backend {
    fn name(&self) -> &'static str {
        "x11"
    }

    fn outputs(&mut self) -> Result<Vec<Output>> {
        let mut count: c_int = 0;
        // SAFETY: display/root valid; get_active = true; count receives length.
        let monitors =
            unsafe { (self.xrandr.XRRGetMonitors)(self.display, self.root, TRUE, &mut count) };
        if monitors.is_null() || count <= 0 {
            return Err(Error::NoOutputs);
        }
        let mut outputs = Vec::with_capacity(count as usize);
        for i in 0..count as isize {
            // SAFETY: i is within [0, count); monitors points to `count` entries.
            let m = unsafe { &*monitors.offset(i) };
            let name = self
                .atom_name(m.name)
                .unwrap_or_else(|| format!("monitor-{i}"));
            outputs.push(Output {
                name,
                x: m.x,
                y: m.y,
                width: m.width.max(0) as u32,
                height: m.height.max(0) as u32,
                scale: 1.0,
                refresh_hz: None,
            });
        }
        // SAFETY: monitors was allocated by XRRGetMonitors.
        unsafe { (self.xrandr.XRRFreeMonitors)(monitors) };
        Ok(outputs)
    }

    fn run(self: Box<Self>, plans: Vec<WallpaperPlan>) -> Result<()> {
        util::install_signal_handlers();

        let mut instances = Vec::new();
        for plan in &plans {
            let (window, surface) = self.create_window(&plan.output)?;
            surface.make_current()?;
            surface.set_swap_interval(0); // we pace manually for multi-monitor.

            let mut player = MpvPlayer::new(&plan.settings, &plan.source)?;
            player.init_render(
                NativeDisplay::X11(self.display as *mut _),
                mpv_get_proc_address,
                None,
            )?;

            instances.push(X11Instance {
                player,
                surface,
                _window: window,
                width: plan.output.width as i32,
                height: plan.output.height as i32,
            });
        }

        // Pace the loop to the fastest desired frame rate across outputs.
        let target_fps = plans
            .iter()
            .map(desired_fps)
            .max()
            .unwrap_or(60)
            .clamp(1, 240);
        let frame_budget = Duration::from_secs_f64(1.0 / target_fps as f64);
        log::info!("X11 render loop at up to {target_fps} fps");

        let mut event: xlib::XEvent = unsafe { std::mem::zeroed() };
        while !util::should_terminate() {
            let start = Instant::now();

            // Drain pending X events (we mostly just keep the queue clear).
            // SAFETY: display valid; event is a valid out-param.
            while unsafe { (self.xlib.XPending)(self.display) } > 0 {
                unsafe { (self.xlib.XNextEvent)(self.display, &mut event) };
            }

            for inst in &instances {
                if inst.player.pump_events() {
                    log::info!("mpv requested shutdown");
                    return Ok(());
                }
                inst.surface.make_current()?;
                inst.player.render(0, inst.width, inst.height)?;
                inst.surface.swap_buffers()?;
            }

            let elapsed = start.elapsed();
            if elapsed < frame_budget {
                std::thread::sleep(frame_budget - elapsed);
            }
        }

        log::info!("Shutting down X11 backend");
        Ok(())
    }
}

/// One output's rendering state. `player` is declared first so it (and its mpv
/// render context) is dropped before the GL surface it draws into.
struct X11Instance {
    player: MpvPlayer,
    surface: GlSurface,
    _window: xlib::Window,
    width: i32,
    height: i32,
}

/// Desired frame rate for a plan: explicit `fps`, else the monitor refresh,
/// else 60.
fn desired_fps(plan: &WallpaperPlan) -> u32 {
    if plan.settings.fps > 0 {
        plan.settings.fps
    } else {
        plan.output
            .refresh_hz
            .map(|r| r.round() as u32)
            .unwrap_or(60)
    }
}
