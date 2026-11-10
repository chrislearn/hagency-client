//! Workspace geometry shared by native pages. Widths are logical pixels;
//! typography remains in Makepad points. Theme density scales spacing only.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PageLayout {
    pub gutter: f64,
    pub gap: f64,
    pub stacked: bool,
    pub compact: bool,
}

impl PageLayout {
    pub fn from_theme(width: f64, theme: &super::Snapshot) -> Self {
        let mut layout = Self::resolve(width, theme.spacing_scale);
        // A package's gutter remains authoritative, including when previewed.
        layout.gutter *= theme.page_gutter / 16.0;
        layout.stacked = width < 960.0 * theme.text_scale.max(1.0);
        layout.compact = width < 640.0 * theme.text_scale.max(1.0);
        layout
    }

    pub fn resolve(width: f64, spacing: f64) -> Self {
        let spacing = if spacing.is_finite() {
            spacing.clamp(0.75, 1.5)
        } else {
            1.0
        };
        Self {
            gutter: (if width < 640.0 { 16.0 } else { 24.0 }) * spacing,
            gap: 16.0 * spacing,
            stacked: width < 960.0,
            compact: width < 640.0,
        }
    }
}
