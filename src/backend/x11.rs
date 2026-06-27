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
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use x11_dl::xfixes;
use x11_dl::xlib::{self, Xlib};
use x11_dl::xrandr::Xrandr;

use crate::backend::{Backend, WallpaperPlan};
use crate::error::{Error, Result};
use crate::ipc::DaemonCommand;
use crate::monitor::Output;
use crate::player::{MpvPlayer, NativeDisplay};
use crate::render::{mpv_get_proc_address, EglDisplay, GlSurface};
use crate::util;

// --- X11 ABI constants (from X.h / Xrandr); not exported by x11-dl. ----------
const ALLOC_NONE: c_int = 0;
const INPUT_OUTPUT: c_uint = 1;
const CW_BACK_PIXEL: c_ulong = 1 << 1;
const CW_BORDER_PIXEL: c_ulong = 1 << 3;
const CW_EVENT_MASK: c_ulong = 1 << 11;
const CW_COLORMAP: c_ulong = 1 << 13;
const EXPOSURE_MASK: c_long = 1 << 15;
const STRUCTURE_NOTIFY_MASK: c_long = 1 << 17;
const VISUAL_ID_MASK: c_long = 0x1;
const PROP_MODE_REPLACE: c_int = 0;
const XA_ATOM: c_ulong = 4;
const XA_CARDINAL: c_ulong = 6;
const ALL_DESKTOPS: c_ulong = 0xFFFF_FFFF;
const TRUE: i32 = 1;
/// Shape extension `ShapeInput` kind (for an empty click-through input region).
const SHAPE_INPUT: c_int = 2;

/// The X11 backend holds the Xlib connection plus the initialised EGL display.
pub struct X11Backend {
    xlib: Xlib,
    xrandr: Xrandr,
    /// Optional XFixes handle, used to make the wallpaper window click-through.
    xfixes: Option<xfixes::Xlib>,
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

        // XFixes is used to make the wallpaper window click-through. It's
        // optional: if it can't be loaded/initialised we simply skip that.
        let xfixes = xfixes::Xlib::open().ok();
        if let Some(xf) = &xfixes {
            let mut major: c_int = 4;
            let minor: c_int = 0;
            // SAFETY: display is valid; initialises the XFixes extension so the
            // shape-region calls below are accepted by the server.
            unsafe { (xf.XFixesQueryVersion)(display, &mut major, &minor) };
        }

        let egl = EglDisplay::new(display as *mut _)?;

        Ok(X11Backend {
            xlib,
            xrandr,
            xfixes,
            display,
            root,
            egl,
        })
    }

    /// Make `window` ignore all pointer/keyboard input by giving it an empty
    /// input shape, so clicks fall through to the desktop beneath it. No-op if
    /// XFixes is unavailable.
    fn set_input_passthrough(&self, window: xlib::Window) {
        let Some(xf) = &self.xfixes else {
            return;
        };
        // SAFETY: display/window valid. An empty region (no rectangles) used as
        // the ShapeInput region makes the window transparent to input.
        unsafe {
            let region = (xf.XFixesCreateRegion)(self.display, ptr::null_mut(), 0);
            (xf.XFixesSetWindowShapeRegion)(self.display, window, SHAPE_INPUT, 0, 0, region);
            (xf.XFixesDestroyRegion)(self.display, region);
        }
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

        // Note: we deliberately leave `override_redirect` false so the window
        // manager *manages* this window and honours the desktop-layer hints we
        // set below (`_NET_WM_WINDOW_TYPE_DESKTOP` + `_NET_WM_STATE_BELOW`).
        // An override-redirect window would float on top under compositing WMs
        // like KWin instead of sitting behind everything as a wallpaper.
        let mut attrs: xlib::XSetWindowAttributes = unsafe { std::mem::zeroed() };
        attrs.background_pixel = 0;
        attrs.border_pixel = 0;
        attrs.colormap = colormap;
        attrs.event_mask = EXPOSURE_MASK | STRUCTURE_NOTIFY_MASK;
        let valuemask = CW_BACK_PIXEL | CW_BORDER_PIXEL | CW_COLORMAP | CW_EVENT_MASK;

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

        self.set_desktop_hints(window);
        self.set_window_name(window, "desktobian");
        // Let desktop clicks (icons, right-click menu) pass through to the DE.
        self.set_input_passthrough(window);

        // SAFETY: window is valid. Map it, then raise it to the top of the
        // desktop layer so it draws over the DE's own wallpaper window while
        // the DESKTOP window type keeps it below normal application windows.
        unsafe {
            (self.xlib.XMapWindow)(self.display, window);
            (self.xlib.XRaiseWindow)(self.display, window);
            (self.xlib.XSync)(self.display, 0);
        }

        // SAFETY: `window` is a live X window of a visual compatible with the
        // EGL config; it outlives the surface (dropped together by the caller).
        let surface = unsafe { self.egl.create_surface(window as *mut _) }?;
        Ok((window, surface))
    }

    /// Apply the EWMH hints that make a window behave as a desktop background:
    /// desktop window type, "below" + skip-taskbar/pager + sticky states, and
    /// presence on all virtual desktops.
    fn set_desktop_hints(&self, window: xlib::Window) {
        self.set_atom_property(
            window,
            "_NET_WM_WINDOW_TYPE",
            &["_NET_WM_WINDOW_TYPE_DESKTOP"],
        );
        // NB: we intentionally do *not* set `_NET_WM_STATE_BELOW`. The DESKTOP
        // window type already keeps us beneath normal windows; `BELOW` would
        // additionally push us under the desktop environment's own wallpaper
        // window (e.g. plasmashell on KDE), hiding the video entirely.
        self.set_atom_property(
            window,
            "_NET_WM_STATE",
            &[
                "_NET_WM_STATE_SKIP_TASKBAR",
                "_NET_WM_STATE_SKIP_PAGER",
                "_NET_WM_STATE_STICKY",
            ],
        );
        self.set_cardinal_property(window, "_NET_WM_DESKTOP", ALL_DESKTOPS);
    }

    /// Set a property to an array of atoms (interned by name).
    fn set_atom_property(&self, window: xlib::Window, prop: &str, value_names: &[&str]) {
        let prop_atom = self.intern(prop);
        if prop_atom == 0 {
            return;
        }
        let atoms: Vec<xlib::Atom> = value_names.iter().map(|n| self.intern(n)).collect();
        if atoms.contains(&0) {
            return;
        }
        // SAFETY: valid display/window/atom; `atoms` holds `len` 32-bit ATOMs.
        unsafe {
            (self.xlib.XChangeProperty)(
                self.display,
                window,
                prop_atom,
                XA_ATOM,
                32,
                PROP_MODE_REPLACE,
                atoms.as_ptr() as *const c_uchar,
                atoms.len() as c_int,
            );
        }
    }

    /// Set a single 32-bit CARDINAL property.
    fn set_cardinal_property(&self, window: xlib::Window, prop: &str, value: c_ulong) {
        let prop_atom = self.intern(prop);
        if prop_atom == 0 {
            return;
        }
        // SAFETY: valid display/window/atom; one 32-bit CARDINAL element.
        unsafe {
            (self.xlib.XChangeProperty)(
                self.display,
                window,
                prop_atom,
                XA_CARDINAL,
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

    fn run(
        self: Box<Self>,
        plans: Vec<WallpaperPlan>,
        commands: Receiver<DaemonCommand>,
    ) -> Result<()> {
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
            // Load media only after the render context exists (see MpvPlayer::new).
            player.load_source(&plan.source)?;

            instances.push(X11Instance {
                player,
                surface,
                output_name: plan.output.name.clone(),
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

            // Apply any pending IPC control commands.
            while let Ok(cmd) = commands.try_recv() {
                let response = crate::ipc::process(&cmd.request, &instances);
                let _ = cmd.reply.try_send(response);
            }

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
    output_name: String,
    _window: xlib::Window,
    width: i32,
    height: i32,
}

impl crate::ipc::Controllable for X11Instance {
    fn output_name(&self) -> &str {
        &self.output_name
    }
    fn player(&self) -> Option<&MpvPlayer> {
        Some(&self.player)
    }
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
