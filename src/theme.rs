/// The visual theme of the application.
///
/// The default look is the light "Swiss minimalist" palette. Selecting
/// [`Theme::Dark`] adds the `dark` class to the root wrapper, which re-maps the
/// Tailwind color tokens to their dark equivalents (see `input.css`).
///
/// The active theme is persisted in the local settings table so it is restored
/// on the next launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    #[default]
    Light,
    Dark,
}

impl Theme {
    /// Stable string used for persistence in the settings table.
    pub fn as_str(self) -> &'static str {
        match self {
            Theme::Light => "light",
            Theme::Dark => "dark",
        }
    }

    /// Parse a persisted value, falling back to the default theme.
    pub fn from_str(value: &str) -> Self {
        match value {
            "dark" => Theme::Dark,
            _ => Theme::Light,
        }
    }

    /// The CSS class applied to the root wrapper for this theme.
    pub fn root_class(self) -> &'static str {
        match self {
            Theme::Dark => "dark",
            Theme::Light => "",
        }
    }

    /// The opposite theme, used by the toggle control.
    pub fn toggled(self) -> Self {
        match self {
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::Light,
        }
    }

    pub fn is_dark(self) -> bool {
        matches!(self, Theme::Dark)
    }
}
