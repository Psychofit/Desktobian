//! Backend-agnostic description of a display output (monitor).
//!
//! Both the X11 (Xrandr) and Wayland (`wl_output`) backends translate their
//! native output information into this common shape so the rest of the engine
//! never has to care which display server it is running on.

/// A single monitor / display output.
#[derive(Debug, Clone, PartialEq)]
pub struct Output {
    /// Connector / output name, e.g. `eDP-1`, `HDMI-A-1`, `DP-2`.
    ///
    /// This is the stable handle users refer to in their config (`[output.HDMI-A-1]`).
    pub name: String,
    /// Position of the top-left corner in the global compositor/X coordinate
    /// space, in physical pixels.
    pub x: i32,
    pub y: i32,
    /// Resolution in physical pixels.
    pub width: u32,
    pub height: u32,
    /// Fractional/integer scale factor (1.0 = unscaled, 2.0 = HiDPI).
    pub scale: f64,
    /// Refresh rate in Hz, if the backend could determine it. Used to cap the
    /// render loop sensibly.
    pub refresh_hz: Option<f64>,
}

impl Output {
    /// Logical width after applying the scale factor (rounded).
    pub fn logical_width(&self) -> u32 {
        ((self.width as f64) / self.scale).round() as u32
    }

    /// Logical height after applying the scale factor (rounded).
    pub fn logical_height(&self) -> u32 {
        ((self.height as f64) / self.scale).round() as u32
    }

    /// A human-friendly one-line summary for `--list-outputs` and logs.
    pub fn summary(&self) -> String {
        let hz = self
            .refresh_hz
            .map(|r| format!(" @{r:.0}Hz"))
            .unwrap_or_default();
        format!(
            "{}: {}x{}+{}+{} (scale {}){}",
            self.name, self.width, self.height, self.x, self.y, self.scale, hz
        )
    }
}
