//! Image rendering for terminal screenshots.

use std::path::Path;

use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use font_kit::{family_name::FamilyName, handle::Handle, source::SystemSource};
use image::{ImageBuffer, Rgba, RgbaImage};
use imageproc::drawing::{draw_filled_rect_mut, draw_text_mut};
use imageproc::rect::Rect;

use crate::error::{Result, TermwrightError};
use crate::screen::Screen;

use super::colors::color_to_rgba;

/// Configuration for screenshot rendering.
#[derive(Debug, Clone)]
pub struct ScreenshotConfig {
    /// Font family name (None = system monospace).
    pub font_name: Option<String>,
    /// Font size in pixels.
    pub font_size: f32,
    /// Line height as a multiplier of font size.
    pub line_height: f32,
}

impl Default for ScreenshotConfig {
    fn default() -> Self {
        Self {
            font_name: None,
            font_size: 14.0,
            line_height: 1.2,
        }
    }
}

/// A screenshot of the terminal screen.
pub struct Screenshot {
    screen: Screen,
    config: ScreenshotConfig,
}

impl Screenshot {
    /// Create a new screenshot from a screen.
    pub fn new(screen: Screen) -> Self {
        Self {
            screen,
            config: ScreenshotConfig::default(),
        }
    }

    /// Create a screenshot with custom configuration.
    pub fn with_config(screen: Screen, config: ScreenshotConfig) -> Self {
        Self { screen, config }
    }

    /// Set the font family.
    pub fn font(mut self, name: &str, size: f32) -> Self {
        self.config.font_name = Some(name.to_string());
        self.config.font_size = size;
        self
    }

    /// Set the line height multiplier.
    pub fn line_height(mut self, height: f32) -> Self {
        self.config.line_height = height;
        self
    }

    /// Render to an image buffer.
    pub fn render(&self) -> Result<RgbaImage> {
        render_screen(&self.screen, &self.config)
    }

    /// Save the screenshot to a file.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let image = self.render()?;
        image
            .save(path)
            .map_err(|e| TermwrightError::Image(e.to_string()))?;
        Ok(())
    }

    /// Get the screenshot as PNG bytes.
    pub fn to_png(&self) -> Result<Vec<u8>> {
        let image = self.render()?;
        let mut bytes = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut bytes);
        image
            .write_with_encoder(encoder)
            .map_err(|e| TermwrightError::Image(e.to_string()))?;
        Ok(bytes)
    }

    /// Get the underlying image buffer.
    pub fn to_image(&self) -> Result<RgbaImage> {
        self.render()
    }
}

/// Render a screen to an image.
fn render_screen(screen: &Screen, config: &ScreenshotConfig) -> Result<RgbaImage> {
    // Load font
    let source = SystemSource::new();
    let handle = match &config.font_name {
        Some(name) => source
            .select_best_match(&[FamilyName::Title(name.clone())], &Default::default())
            .map_err(|e| TermwrightError::Font(e.to_string()))?,
        None => source
            .select_best_match(&[FamilyName::Monospace], &Default::default())
            .map_err(|e| TermwrightError::Font(e.to_string()))?,
    };

    let font_data = match handle {
        Handle::Path { path, .. } => {
            std::fs::read(path).map_err(|e| TermwrightError::Font(e.to_string()))?
        }
        Handle::Memory { bytes, .. } => bytes.to_vec(),
    };

    let font =
        FontRef::try_from_slice(&font_data).map_err(|e| TermwrightError::Font(e.to_string()))?;

    let scale = PxScale::from(config.font_size);
    let scaled_font = font.as_scaled(scale);

    // Calculate dimensions
    let line_height = config.font_size * config.line_height;
    let glyph = font.glyph_id('M');
    let char_width = scaled_font.h_advance(glyph);

    let width = screen.size.cols as f32 * char_width;
    let height = screen.size.rows as f32 * line_height;

    // Create image with black background
    let mut image: RgbaImage = ImageBuffer::new(width.ceil() as u32, height.ceil() as u32);
    let default_bg = Rgba([0, 0, 0, 255]);
    let full_rect = Rect::at(0, 0).of_size(width.ceil() as u32, height.ceil() as u32);
    draw_filled_rect_mut(&mut image, full_rect, default_bg);

    // Render each cell
    for (row_idx, row) in screen.raw_cells().iter().enumerate() {
        let y = row_idx as f32 * line_height;

        for (col_idx, cell) in row.iter().enumerate() {
            let x = col_idx as f32 * char_width;

            // Determine colors (handle inverse)
            let (fg_color, bg_color) = if cell.attrs.inverse {
                (
                    color_to_rgba(&cell.bg, false),
                    color_to_rgba(&cell.fg, true),
                )
            } else {
                (
                    color_to_rgba(&cell.fg, true),
                    color_to_rgba(&cell.bg, false),
                )
            };

            // Draw background
            let rect = Rect::at(x.round() as i32, y.round() as i32)
                .of_size(char_width.ceil() as u32, line_height.ceil() as u32);
            draw_filled_rect_mut(&mut image, rect, bg_color);

            // Draw character (skip spaces for performance)
            if cell.char != ' ' {
                draw_text_mut(
                    &mut image,
                    fg_color,
                    x as i32,
                    y as i32,
                    scale,
                    &font,
                    &cell.char.to_string(),
                );
            }
        }
    }

    Ok(image)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screenshot_config_default() {
        let config = ScreenshotConfig::default();
        assert_eq!(config.font_size, 14.0);
        assert_eq!(config.line_height, 1.2);
        assert!(config.font_name.is_none());
    }
}
