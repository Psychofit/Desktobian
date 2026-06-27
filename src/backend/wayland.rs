//! Wayland backend using the `wlr-layer-shell` protocol.
//!
//! For each output we create a layer surface on the **background** layer,
//! anchored to all four edges so it fills the monitor, and render the wallpaper
//! into it via an EGL window surface driven by mpv.
//!
//! Rendering is paced by the compositor: after each frame we request a
//! `wl_surface.frame` callback, and [`CompositorHandler::frame`] draws the next
//! one. This gives smooth, vsync-aligned playback that automatically throttles
//! when the wallpaper is fully occluded.
//!
//! Works on wlroots-based compositors (Sway, Hyprland, river, Wayfire, …).

use std::os::raw::c_void;
use std::sync::mpsc::Receiver;

use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_layer, delegate_output, delegate_registry,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    shell::{
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
        WaylandSurface,
    },
};
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_output, wl_surface},
    Connection, EventQueue, Proxy, QueueHandle,
};
use wayland_egl::WlEglSurface;

use crate::backend::{Backend, WallpaperPlan};
use crate::error::{Error, Result};
use crate::ipc::DaemonCommand;
use crate::monitor::Output;
use crate::player::{MpvPlayer, NativeDisplay};
use crate::render::{mpv_get_proc_address, EglDisplay, GlSurface};
use crate::util;

/// Live GL state for one output, created lazily on the first `configure`.
///
/// Field order matters for teardown: `player` (and its mpv render context) is
/// dropped first, then the EGL `surface`, then the `egl_window` it was built on.
struct WlGl {
    player: MpvPlayer,
    surface: GlSurface,
    egl_window: WlEglSurface,
}

/// One output's wallpaper surface.
struct WlInstance {
    layer: LayerSurface,
    output_name: String,
    plan_index: usize,
    width: i32,
    height: i32,
    gl: Option<WlGl>,
}

impl crate::ipc::Controllable for WlInstance {
    fn output_name(&self) -> &str {
        &self.output_name
    }
    fn player(&self) -> Option<&MpvPlayer> {
        self.gl.as_ref().map(|g| &g.player)
    }
}

/// The dispatched Wayland application state.
struct WaylandState {
    registry_state: RegistryState,
    output_state: OutputState,
    compositor_state: CompositorState,
    layer_shell: LayerShell,
    qh: QueueHandle<WaylandState>,
    conn: Connection,
    egl: EglDisplay,
    plans: Vec<WallpaperPlan>,
    instances: Vec<WlInstance>,
    exit: bool,
}

/// The Wayland backend: owns the event queue and the dispatched state.
pub struct WaylandBackend {
    event_queue: EventQueue<WaylandState>,
    state: WaylandState,
}

impl WaylandBackend {
    /// Connect to the compositor, bind the required globals, initialise EGL, and
    /// roundtrip once so output information is available.
    pub fn connect() -> Result<Self> {
        let conn = Connection::connect_to_env()
            .map_err(|e| Error::Wayland(format!("cannot connect to Wayland: {e}")))?;

        let (globals, mut event_queue) = registry_queue_init::<WaylandState>(&conn)
            .map_err(|e| Error::Wayland(format!("registry init failed: {e}")))?;
        let qh = event_queue.handle();

        let compositor_state = CompositorState::bind(&globals, &qh)
            .map_err(|e| Error::Wayland(format!("wl_compositor unavailable: {e}")))?;
        let layer_shell = LayerShell::bind(&globals, &qh).map_err(|e| {
            Error::Wayland(format!(
                "wlr-layer-shell unavailable ({e}); your compositor may not support it \
                 (GNOME/KDE Wayland do not — use the X11 backend there)"
            ))
        })?;

        // SAFETY: the wl_display pointer is owned by `conn`, which lives in the
        // returned backend alongside the EGL display.
        let display_ptr = conn.backend().display_ptr() as *mut c_void;
        let egl = EglDisplay::new(display_ptr)?;

        let mut state = WaylandState {
            registry_state: RegistryState::new(&globals),
            output_state: OutputState::new(&globals, &qh),
            compositor_state,
            layer_shell,
            qh: qh.clone(),
            conn: conn.clone(),
            egl,
            plans: Vec::new(),
            instances: Vec::new(),
            exit: false,
        };

        // Let output globals settle so `outputs()` is populated.
        event_queue
            .roundtrip(&mut state)
            .map_err(|e| Error::Wayland(format!("initial roundtrip failed: {e}")))?;

        Ok(WaylandBackend { event_queue, state })
    }
}

impl Backend for WaylandBackend {
    fn name(&self) -> &'static str {
        "wayland"
    }

    fn outputs(&mut self) -> Result<Vec<Output>> {
        let mut outputs = Vec::new();
        for wl_output in self.state.output_state.outputs() {
            let Some(info) = self.state.output_state.info(&wl_output) else {
                continue;
            };
            let name = info
                .name
                .clone()
                .unwrap_or_else(|| format!("wl-output-{}", info.id));
            let (width, height) = info
                .logical_size
                .or_else(|| info.modes.iter().find(|m| m.current).map(|m| m.dimensions))
                .unwrap_or((0, 0));
            let (x, y) = info.logical_position.unwrap_or(info.location);
            let refresh = info
                .modes
                .iter()
                .find(|m| m.current)
                .map(|m| m.refresh_rate as f64 / 1000.0);
            outputs.push(Output {
                name,
                x,
                y,
                width: width.max(0) as u32,
                height: height.max(0) as u32,
                scale: info.scale_factor.max(1) as f64,
                refresh_hz: refresh,
            });
        }
        Ok(outputs)
    }

    fn run(
        self: Box<Self>,
        plans: Vec<WallpaperPlan>,
        commands: Receiver<DaemonCommand>,
    ) -> Result<()> {
        util::install_signal_handlers();
        let WaylandBackend {
            mut event_queue,
            mut state,
        } = *self;
        let qh = state.qh.clone();

        // Match each plan to its wl_output (collect first to avoid borrow churn).
        let mut matched: Vec<(usize, wl_output::WlOutput)> = Vec::new();
        for (idx, plan) in plans.iter().enumerate() {
            let found = state.output_state.outputs().find(|o| {
                state.output_state.info(o).and_then(|i| i.name).as_deref()
                    == Some(plan.output.name.as_str())
            });
            match found {
                Some(o) => matched.push((idx, o)),
                None => log::warn!("output {} vanished before setup", plan.output.name),
            }
        }

        // Create one background layer surface per matched output.
        for (idx, output) in matched {
            let surface = state.compositor_state.create_surface(&qh);
            let layer = state.layer_shell.create_layer_surface(
                &qh,
                surface,
                Layer::Background,
                Some("desktobian"),
                Some(&output),
            );
            layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
            // Render *under* panels/bars rather than reserving space.
            layer.set_exclusive_zone(-1);
            layer.set_keyboard_interactivity(KeyboardInteractivity::None);
            // Initial commit (no buffer) so the compositor sends a configure.
            layer.commit();

            state.instances.push(WlInstance {
                layer,
                output_name: plans[idx].output.name.clone(),
                plan_index: idx,
                width: 0,
                height: 0,
                gl: None,
            });
        }
        state.plans = plans;

        log::info!("Wayland render loop started");
        while !state.exit && !util::should_terminate() {
            event_queue
                .blocking_dispatch(&mut state)
                .map_err(|e| Error::Wayland(format!("dispatch failed: {e}")))?;

            // Apply any pending IPC control commands. While a video is playing
            // the compositor delivers frame callbacks continuously, so this runs
            // promptly; a fully-occluded/idle surface may defer until the next
            // event.
            while let Ok(cmd) = commands.try_recv() {
                let response = crate::ipc::process(&cmd.request, &state.instances);
                let _ = cmd.reply.try_send(response);
            }
        }

        log::info!("Shutting down Wayland backend");
        Ok(())
    }
}

impl WaylandState {
    /// Locate the instance owning `surface`, if any.
    fn index_of_surface(&self, surface: &wl_surface::WlSurface) -> Option<usize> {
        self.instances
            .iter()
            .position(|i| i.layer.wl_surface() == surface)
    }

    /// Ensure GL state exists for instance `idx`, sized `w`x`h`, (re)creating or
    /// resizing as needed.
    fn ensure_gl(&mut self, idx: usize, w: i32, h: i32) -> Result<()> {
        if self.instances[idx].gl.is_some() {
            if self.instances[idx].width != w || self.instances[idx].height != h {
                if let Some(gl) = &self.instances[idx].gl {
                    gl.egl_window.resize(w, h, 0, 0);
                }
                self.instances[idx].width = w;
                self.instances[idx].height = h;
            }
            return Ok(());
        }

        let plan_index = self.instances[idx].plan_index;
        let settings = self.plans[plan_index].settings.clone();
        let source = self.plans[plan_index].source.clone();

        let wl_surface = self.instances[idx].layer.wl_surface().clone();
        let egl_window = WlEglSurface::new(wl_surface.id(), w, h)
            .map_err(|e| Error::Wayland(format!("wl_egl_window creation failed: {e}")))?;

        // SAFETY: `egl_window` lives in the returned WlGl, owned by the instance,
        // so its pointer remains valid for the surface's lifetime.
        let surface = unsafe { self.egl.create_surface(egl_window.ptr() as *mut c_void) }?;
        surface.make_current()?;
        surface.set_swap_interval(1); // frame callbacks pace us; vsync is fine.

        let mut player = MpvPlayer::new(&settings, &source)?;
        let display_ptr = self.conn.backend().display_ptr() as *mut c_void;
        player.init_render(
            NativeDisplay::Wayland(display_ptr),
            mpv_get_proc_address,
            None,
        )?;
        // Load media only after the render context exists (see MpvPlayer::new).
        player.load_source(&source)?;

        self.instances[idx].width = w;
        self.instances[idx].height = h;
        self.instances[idx].gl = Some(WlGl {
            player,
            surface,
            egl_window,
        });
        log::debug!("output {idx}: GL surface ready at {w}x{h}");
        Ok(())
    }

    /// Draw one frame for instance `idx` and request the next frame callback.
    fn render_frame(&self, idx: usize) -> Result<()> {
        let inst = &self.instances[idx];
        let Some(gl) = &inst.gl else {
            return Ok(());
        };
        gl.surface.make_current()?;
        gl.player.render(0, inst.width, inst.height)?;
        // Request the next frame *before* the swap commits the surface.
        let wl_surface = inst.layer.wl_surface();
        wl_surface.frame(&self.qh, wl_surface.clone());
        gl.surface.swap_buffers()?;
        Ok(())
    }

    /// Drain mpv events for instance `idx`; set `exit` if mpv shut down.
    fn pump(&mut self, idx: usize) {
        if let Some(gl) = &self.instances[idx].gl {
            if gl.player.pump_events() {
                log::info!("mpv requested shutdown");
                self.exit = true;
            }
        }
    }
}

impl CompositorHandler for WaylandState {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        let Some(idx) = self.index_of_surface(surface) else {
            return;
        };
        self.pump(idx);
        if self.exit {
            return;
        }
        if let Err(e) = self.render_frame(idx) {
            log::error!("render failed: {e}");
            self.exit = true;
        }
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
}

impl LayerShellHandler for WaylandState {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, layer: &LayerSurface) {
        if let Some(idx) = self.index_of_surface(layer.wl_surface()) {
            log::info!("layer surface for output {idx} closed");
            self.instances[idx].gl = None;
        }
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        let Some(idx) = self.index_of_surface(layer.wl_surface()) else {
            return;
        };
        let plan_index = self.instances[idx].plan_index;
        let (mut w, mut h) = configure.new_size;
        if w == 0 {
            w = self.plans[plan_index].output.logical_width();
        }
        if h == 0 {
            h = self.plans[plan_index].output.logical_height();
        }
        let (w, h) = (w.max(1) as i32, h.max(1) as i32);

        if let Err(e) = self.ensure_gl(idx, w, h) {
            log::error!("output {idx}: GL setup failed: {e}");
            self.exit = true;
            return;
        }
        if let Err(e) = self.render_frame(idx) {
            log::error!("output {idx}: initial render failed: {e}");
            self.exit = true;
        }
    }
}

impl OutputHandler for WaylandState {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }
    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
}

impl ProvidesRegistryState for WaylandState {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState];
}

delegate_compositor!(WaylandState);
delegate_output!(WaylandState);
delegate_layer!(WaylandState);
delegate_registry!(WaylandState);
