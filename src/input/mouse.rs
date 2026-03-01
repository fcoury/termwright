use std::str::FromStr;

/// Mouse buttons understood by the daemon API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
}

impl FromStr for MouseButton {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "left" => Ok(MouseButton::Left),
            "middle" => Ok(MouseButton::Middle),
            "right" => Ok(MouseButton::Right),
            other => Err(format!("unknown mouse button: {other}")),
        }
    }
}

impl std::fmt::Display for MouseButton {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MouseButton::Left => write!(f, "left"),
            MouseButton::Middle => write!(f, "middle"),
            MouseButton::Right => write!(f, "right"),
        }
    }
}

impl MouseButton {
    pub(crate) fn press_code(self) -> u8 {
        match self {
            MouseButton::Left => 0,
            MouseButton::Middle => 1,
            MouseButton::Right => 2,
        }
    }
}

/// Scroll wheel direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    Up,
    Down,
}

impl FromStr for ScrollDirection {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "up" => Ok(ScrollDirection::Up),
            "down" => Ok(ScrollDirection::Down),
            other => Err(format!("unknown scroll direction: {other}")),
        }
    }
}

impl std::fmt::Display for ScrollDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScrollDirection::Up => write!(f, "up"),
            ScrollDirection::Down => write!(f, "down"),
        }
    }
}

impl ScrollDirection {
    /// SGR mouse code for scroll events.
    /// Scroll up = 64, scroll down = 65.
    pub(crate) fn sgr_code(self) -> u8 {
        match self {
            ScrollDirection::Up => 64,
            ScrollDirection::Down => 65,
        }
    }
}
