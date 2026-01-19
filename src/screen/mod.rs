//! Screen state and querying.

use serde::{Deserialize, Serialize};

/// Position on the terminal screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    /// Row (0-indexed from top).
    pub row: u16,
    /// Column (0-indexed from left).
    pub col: u16,
}

impl Position {
    /// Create a new position.
    pub fn new(row: u16, col: u16) -> Self {
        Self { row, col }
    }
}

/// Terminal screen dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Size {
    /// Number of columns.
    pub cols: u16,
    /// Number of rows.
    pub rows: u16,
}

impl Size {
    /// Create a new size.
    pub fn new(cols: u16, rows: u16) -> Self {
        Self { cols, rows }
    }
}

/// Color representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum Color {
    /// Default terminal color.
    Default,
    /// Indexed color (0-255).
    Indexed(u8),
    /// RGB color.
    Rgb(u8, u8, u8),
}

impl Default for Color {
    fn default() -> Self {
        Color::Default
    }
}

impl From<vt100::Color> for Color {
    fn from(color: vt100::Color) -> Self {
        match color {
            vt100::Color::Default => Color::Default,
            vt100::Color::Idx(idx) => Color::Indexed(idx),
            vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
        }
    }
}

/// Cell attributes (bold, italic, etc.).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellAttributes {
    /// Bold text.
    pub bold: bool,
    /// Italic text.
    pub italic: bool,
    /// Underlined text.
    pub underline: bool,
    /// Inverse/reverse video.
    pub inverse: bool,
}

/// A single cell on the terminal screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell {
    /// The character in this cell.
    pub char: char,
    /// Foreground color.
    pub fg: Color,
    /// Background color.
    pub bg: Color,
    /// Text attributes.
    pub attrs: CellAttributes,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            char: ' ',
            fg: Color::Default,
            bg: Color::Default,
            attrs: CellAttributes::default(),
        }
    }
}

/// A snapshot of the terminal screen state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Screen {
    /// Screen dimensions.
    pub size: Size,
    /// Cursor position.
    pub cursor: Position,
    /// Screen content as rows of cells.
    cells: Vec<Vec<Cell>>,
}

impl Screen {
    /// Create a screen snapshot from a vt100 screen.
    pub fn from_vt100(screen: &vt100::Screen) -> Self {
        let (rows, cols) = screen.size();
        let cursor_pos = screen.cursor_position();

        let mut cells = Vec::with_capacity(rows as usize);
        for row in 0..rows {
            let mut row_cells = Vec::with_capacity(cols as usize);
            for col in 0..cols {
                let cell = screen.cell(row, col);
                if let Some(cell) = cell {
                    row_cells.push(Cell {
                        char: cell.contents().chars().next().unwrap_or(' '),
                        fg: cell.fgcolor().into(),
                        bg: cell.bgcolor().into(),
                        attrs: CellAttributes {
                            bold: cell.bold(),
                            italic: cell.italic(),
                            underline: cell.underline(),
                            inverse: cell.inverse(),
                        },
                    });
                } else {
                    row_cells.push(Cell::default());
                }
            }
            cells.push(row_cells);
        }

        Self {
            size: Size::new(cols, rows),
            cursor: Position::new(cursor_pos.0, cursor_pos.1),
            cells,
        }
    }

    /// Get the full screen content as plain text.
    pub fn text(&self) -> String {
        self.cells
            .iter()
            .map(|row| {
                row.iter()
                    .map(|cell| cell.char)
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Get a single line of text (0-indexed).
    pub fn line(&self, row: u16) -> Option<String> {
        self.cells.get(row as usize).map(|row| {
            row.iter()
                .map(|cell| cell.char)
                .collect::<String>()
                .trim_end()
                .to_string()
        })
    }

    /// Check if the screen contains the given text.
    pub fn contains(&self, text: &str) -> bool {
        self.text().contains(text)
    }

    /// Find all occurrences of text on the screen.
    pub fn find_text(&self, pattern: &str) -> Vec<TextMatch> {
        let mut matches = Vec::new();
        for (row_idx, row) in self.cells.iter().enumerate() {
            let line: String = row.iter().map(|c| c.char).collect();
            let mut start = 0;
            while let Some(pos) = line[start..].find(pattern) {
                let col = start + pos;
                matches.push(TextMatch {
                    position: Position::new(row_idx as u16, col as u16),
                    text: pattern.to_string(),
                    length: pattern.len(),
                });
                start = col + 1;
            }
        }
        matches
    }

    /// Get a cell at the given position.
    pub fn cell(&self, row: u16, col: u16) -> Option<&Cell> {
        self.cells
            .get(row as usize)
            .and_then(|r| r.get(col as usize))
    }

    /// Get the cursor position.
    pub fn cursor(&self) -> Position {
        self.cursor
    }

    /// Get screen dimensions.
    pub fn dimensions(&self) -> Size {
        self.size
    }

    /// Serialize to JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Serialize to compact JSON (text only, no cell details).
    pub fn to_json_compact(&self) -> Result<String, serde_json::Error> {
        #[derive(Serialize)]
        struct CompactScreen {
            size: Size,
            cursor: Position,
            lines: Vec<String>,
        }

        let compact = CompactScreen {
            size: self.size,
            cursor: self.cursor,
            lines: self
                .cells
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|c| c.char)
                        .collect::<String>()
                        .trim_end()
                        .to_string()
                })
                .collect(),
        };

        serde_json::to_string_pretty(&compact)
    }

    /// Extract a region of the screen by coordinates.
    ///
    /// Returns a Region that can be used to extract text or cells.
    pub fn region(
        &self,
        row_range: std::ops::Range<u16>,
        col_range: std::ops::Range<u16>,
    ) -> Region {
        Region::from_ranges(
            row_range.start,
            row_range.end,
            col_range.start,
            col_range.end,
        )
    }

    /// Get all cells in a region.
    pub fn cells_in_region(&self, region: &Region) -> Vec<Vec<Cell>> {
        let mut result = Vec::new();
        for row in region.start.row..region.end.row {
            if let Some(row_cells) = self.cells.get(row as usize) {
                let start_col = region.start.col as usize;
                let end_col = (region.end.col as usize).min(row_cells.len());
                if start_col < row_cells.len() {
                    result.push(row_cells[start_col..end_col].to_vec());
                }
            }
        }
        result
    }

    /// Detect boxes drawn with box-drawing characters.
    ///
    /// Returns a list of regions that appear to be bordered boxes.
    pub fn detect_boxes(&self) -> Vec<DetectedBox> {
        let mut boxes = Vec::new();
        let rows = self.size.rows as usize;
        let cols = self.size.cols as usize;

        // Find top-left corners (┌ or ╔ or similar)
        for row in 0..rows {
            for col in 0..cols {
                if let Some(cell) = self.cells.get(row).and_then(|r| r.get(col)) {
                    if is_top_left_corner(cell.char) {
                        // Try to find the corresponding box
                        if let Some(detected) = self.trace_box(row, col) {
                            boxes.push(detected);
                        }
                    }
                }
            }
        }

        boxes
    }

    /// Trace a box starting from a top-left corner.
    fn trace_box(&self, start_row: usize, start_col: usize) -> Option<DetectedBox> {
        let rows = self.size.rows as usize;
        let cols = self.size.cols as usize;

        // Find the top-right corner by following horizontal lines
        let mut end_col = start_col + 1;
        while end_col < cols {
            if let Some(cell) = self.cells.get(start_row).and_then(|r| r.get(end_col)) {
                if is_top_right_corner(cell.char) {
                    break;
                } else if !is_horizontal_line(cell.char) {
                    return None;
                }
            } else {
                return None;
            }
            end_col += 1;
        }

        if end_col >= cols {
            return None;
        }

        // Find the bottom-right corner by following vertical lines
        let mut end_row = start_row + 1;
        while end_row < rows {
            if let Some(cell) = self.cells.get(end_row).and_then(|r| r.get(end_col)) {
                if is_bottom_right_corner(cell.char) {
                    break;
                } else if !is_vertical_line(cell.char) {
                    return None;
                }
            } else {
                return None;
            }
            end_row += 1;
        }

        if end_row >= rows {
            return None;
        }

        // Verify bottom-left corner
        if let Some(cell) = self.cells.get(end_row).and_then(|r| r.get(start_col)) {
            if !is_bottom_left_corner(cell.char) {
                return None;
            }
        } else {
            return None;
        }

        // Verify bottom horizontal line
        for col in (start_col + 1)..end_col {
            if let Some(cell) = self.cells.get(end_row).and_then(|r| r.get(col)) {
                if !is_horizontal_line(cell.char) {
                    return None;
                }
            } else {
                return None;
            }
        }

        // Verify left vertical line
        for row in (start_row + 1)..end_row {
            if let Some(cell) = self.cells.get(row).and_then(|r| r.get(start_col)) {
                if !is_vertical_line(cell.char) {
                    return None;
                }
            } else {
                return None;
            }
        }

        Some(DetectedBox {
            region: Region::from_ranges(
                start_row as u16,
                (end_row + 1) as u16,
                start_col as u16,
                (end_col + 1) as u16,
            ),
            inner_region: Region::from_ranges(
                (start_row + 1) as u16,
                end_row as u16,
                (start_col + 1) as u16,
                end_col as u16,
            ),
            style: BoxStyle::Single, // Could detect double lines too
        })
    }

    /// Get the raw cells array (for advanced processing).
    pub fn raw_cells(&self) -> &Vec<Vec<Cell>> {
        &self.cells
    }

    /// Iterate over rows.
    pub fn rows(&self) -> impl Iterator<Item = &Vec<Cell>> {
        self.cells.iter()
    }

    /// Find text matching a regex pattern.
    pub fn find_pattern(&self, pattern: &str) -> Result<Vec<TextMatch>, regex::Error> {
        let re = regex::Regex::new(pattern)?;
        let mut matches = Vec::new();

        for (row_idx, row) in self.cells.iter().enumerate() {
            let line: String = row.iter().map(|c| c.char).collect();
            for mat in re.find_iter(&line) {
                matches.push(TextMatch {
                    position: Position::new(row_idx as u16, mat.start() as u16),
                    text: mat.as_str().to_string(),
                    length: mat.len(),
                });
            }
        }

        Ok(matches)
    }
}

/// A detected box on the screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedBox {
    /// The full region including borders.
    pub region: Region,
    /// The inner region (content area, excluding borders).
    pub inner_region: Region,
    /// The box style (single or double lines).
    pub style: BoxStyle,
}

impl DetectedBox {
    /// Extract the text content inside the box.
    pub fn content(&self, screen: &Screen) -> String {
        self.inner_region.extract_text(screen)
    }
}

/// Style of box-drawing characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxStyle {
    /// Single line box (─, │, ┌, ┐, └, ┘)
    Single,
    /// Double line box (═, ║, ╔, ╗, ╚, ╝)
    Double,
    /// Mixed or unknown style
    Mixed,
}

// Box-drawing character detection helpers

fn is_top_left_corner(c: char) -> bool {
    matches!(c, '┌' | '╔' | '┏' | '╭' | '+')
}

fn is_top_right_corner(c: char) -> bool {
    matches!(c, '┐' | '╗' | '┓' | '╮' | '+')
}

fn is_bottom_left_corner(c: char) -> bool {
    matches!(c, '└' | '╚' | '┗' | '╰' | '+')
}

fn is_bottom_right_corner(c: char) -> bool {
    matches!(c, '┘' | '╝' | '┛' | '╯' | '+')
}

fn is_horizontal_line(c: char) -> bool {
    matches!(c, '─' | '═' | '━' | '-')
}

fn is_vertical_line(c: char) -> bool {
    matches!(c, '│' | '║' | '┃' | '|')
}

/// A text match found on the screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextMatch {
    /// Position of the match.
    pub position: Position,
    /// The matched text.
    pub text: String,
    /// Length of the match in characters.
    pub length: usize,
}

/// A rectangular region of the screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    /// Top-left corner.
    pub start: Position,
    /// Bottom-right corner (exclusive).
    pub end: Position,
}

impl Region {
    /// Create a new region.
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    /// Create a region from coordinate ranges.
    pub fn from_ranges(row_start: u16, row_end: u16, col_start: u16, col_end: u16) -> Self {
        Self {
            start: Position::new(row_start, col_start),
            end: Position::new(row_end, col_end),
        }
    }

    /// Extract text from this region of a screen.
    pub fn extract_text(&self, screen: &Screen) -> String {
        let mut lines = Vec::new();
        for row in self.start.row..self.end.row {
            if let Some(row_cells) = screen.cells.get(row as usize) {
                let start_col = self.start.col as usize;
                let end_col = (self.end.col as usize).min(row_cells.len());
                if start_col < row_cells.len() {
                    let line: String = row_cells[start_col..end_col]
                        .iter()
                        .map(|c| c.char)
                        .collect();
                    lines.push(line.trim_end().to_string());
                }
            }
        }
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position() {
        let pos = Position::new(5, 10);
        assert_eq!(pos.row, 5);
        assert_eq!(pos.col, 10);
    }

    #[test]
    fn test_color_default() {
        assert_eq!(Color::default(), Color::Default);
    }
}
