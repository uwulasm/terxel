//! Tiny terminal pixel drawing built on ANSI truecolor and half-block cells.
//!
//! A terminal cell is taller than it is wide in most fonts. Terxels stores a
//! logical pixel grid, then renders every pair of vertical pixels as one
//! terminal character using foreground and background colors.
//!
//! ```
//! use terxels::{Canvas, Color};
//!
//! let mut canvas = Canvas::new(16, 16);
//! canvas.set_pixel(3, 4, Color::rgb(255, 80, 120));
//! print!("{}", canvas.render());
//! ```

use std::fmt::Write as _;
use std::io::{self, Write};

const RESET: &str = "\x1b[39m\x1b[49m";
const UPPER_HALF_BLOCK: char = '\u{2580}';
const LOWER_HALF_BLOCK: char = '\u{2584}';

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

impl From<(u8, u8, u8)> for Color {
    fn from((r, g, b): (u8, u8, u8)) -> Self {
        Self::rgb(r, g, b)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Cell {
    top: Option<Color>,
    bottom: Option<Color>,
}

impl Cell {
    fn render_into(self, output: &mut String) {
        match (self.top, self.bottom) {
            (None, None) => {
                output.push_str(RESET);
                output.push(' ');
            }
            (Some(top), None) => {
                let _ = write!(
                    output,
                    "\x1b[49;38;2;{};{};{}m{}",
                    top.r, top.g, top.b, UPPER_HALF_BLOCK
                );
            }
            (None, Some(bottom)) => {
                let _ = write!(
                    output,
                    "\x1b[49;38;2;{};{};{}m{}",
                    bottom.r, bottom.g, bottom.b, LOWER_HALF_BLOCK
                );
            }
            (Some(top), Some(bottom)) => {
                let _ = write!(
                    output,
                    "\x1b[38;2;{};{};{};48;2;{};{};{}m{}",
                    bottom.r, bottom.g, bottom.b, top.r, top.g, top.b, LOWER_HALF_BLOCK
                );
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Canvas {
    width: usize,
    height: usize,
    scale: usize,
    columns: usize,
    rows: usize,
    cells: Vec<Cell>,
}

impl Canvas {
    pub fn new(width: usize, height: usize) -> Self {
        Self::with_scale(width, height, 1)
    }

    pub fn with_scale(width: usize, height: usize, scale: usize) -> Self {
        assert!(scale > 0, "scale must be greater than zero");

        let columns = width.checked_mul(scale).expect("canvas width overflowed");
        let scaled_height = height.checked_mul(scale).expect("canvas height overflowed");
        let rows = scaled_height.div_ceil(2);
        let cell_count = columns
            .checked_mul(rows)
            .expect("canvas cell count overflowed");

        Self {
            width,
            height,
            scale,
            columns,
            rows,
            cells: vec![Cell::default(); cell_count],
        }
    }

    pub const fn width(&self) -> usize {
        self.width
    }

    pub const fn height(&self) -> usize {
        self.height
    }

    pub const fn scale(&self) -> usize {
        self.scale
    }

    pub const fn terminal_width(&self) -> usize {
        self.columns
    }

    pub const fn terminal_height(&self) -> usize {
        self.rows
    }

    pub fn clear(&mut self) {
        self.cells.fill(Cell::default());
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, color: impl Into<Color>) {
        if x >= self.width || y >= self.height {
            return;
        }

        let color = color.into();
        let scaled_x = x * self.scale;
        let scaled_y = y * self.scale;

        for y_offset in 0..self.scale {
            for x_offset in 0..self.scale {
                self.set_scaled_pixel(scaled_x + x_offset, scaled_y + y_offset, color);
            }
        }
    }

    pub fn render(&self) -> String {
        let mut output = String::new();

        for row in 0..self.rows {
            for column in 0..self.columns {
                self.cells[row * self.columns + column].render_into(&mut output);
            }

            output.push_str(RESET);

            if row + 1 < self.rows {
                output.push('\n');
            }
        }

        output
    }

    pub fn draw(&self, mut writer: impl Write) -> io::Result<()> {
        writer.write_all(self.render().as_bytes())
    }

    fn set_scaled_pixel(&mut self, x: usize, y: usize, color: Color) {
        let cell = &mut self.cells[(y / 2) * self.columns + x];

        if y.is_multiple_of(2) {
            cell.top = Some(color);
        } else {
            cell.bottom = Some(color);
        }
    }
}

pub fn set_cursor_position(mut writer: impl Write, column: usize, row: usize) -> io::Result<()> {
    write!(writer, "\x1b[{row};{column}H")
}

pub fn hide_cursor(mut writer: impl Write) -> io::Result<()> {
    writer.write_all(b"\x1b[?25l")
}

pub fn show_cursor(mut writer: impl Write) -> io::Result<()> {
    writer.write_all(b"\x1b[?25h")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_logical_and_terminal_dimensions() {
        let canvas = Canvas::with_scale(10, 7, 3);

        assert_eq!(canvas.width(), 10);
        assert_eq!(canvas.height(), 7);
        assert_eq!(canvas.scale(), 3);
        assert_eq!(canvas.terminal_width(), 30);
        assert_eq!(canvas.terminal_height(), 11);
    }

    #[test]
    fn renders_empty_pixels_as_reset_spaces() {
        let canvas = Canvas::new(2, 2);

        assert_eq!(
            canvas.render(),
            "\x1b[39m\x1b[49m \x1b[39m\x1b[49m \x1b[39m\x1b[49m"
        );
    }

    #[test]
    fn renders_top_pixel_with_upper_half_block() {
        let mut canvas = Canvas::new(1, 1);

        canvas.set_pixel(0, 0, Color::rgb(1, 2, 3));

        assert_eq!(
            canvas.render(),
            "\x1b[49;38;2;1;2;3m\u{2580}\x1b[39m\x1b[49m"
        );
    }

    #[test]
    fn renders_bottom_pixel_with_lower_half_block() {
        let mut canvas = Canvas::new(1, 2);

        canvas.set_pixel(0, 1, Color::rgb(4, 5, 6));

        assert_eq!(
            canvas.render(),
            "\x1b[49;38;2;4;5;6m\u{2584}\x1b[39m\x1b[49m"
        );
    }

    #[test]
    fn renders_two_pixels_in_one_terminal_cell() {
        let mut canvas = Canvas::new(1, 2);

        canvas.set_pixel(0, 0, Color::rgb(1, 2, 3));
        canvas.set_pixel(0, 1, Color::rgb(4, 5, 6));

        assert_eq!(
            canvas.render(),
            "\x1b[38;2;4;5;6;48;2;1;2;3m\u{2584}\x1b[39m\x1b[49m"
        );
    }

    #[test]
    fn scaled_pixels_fill_multiple_terminal_cells() {
        let mut canvas = Canvas::with_scale(1, 1, 2);

        canvas.set_pixel(0, 0, (7, 8, 9));

        assert_eq!(
            canvas.render(),
            "\x1b[38;2;7;8;9;48;2;7;8;9m\u{2584}\x1b[38;2;7;8;9;48;2;7;8;9m\u{2584}\x1b[39m\x1b[49m"
        );
    }

    #[test]
    fn ignores_out_of_bounds_pixels() {
        let mut canvas = Canvas::new(1, 1);

        canvas.set_pixel(1, 0, Color::rgb(255, 0, 0));
        canvas.set_pixel(0, 1, Color::rgb(0, 255, 0));

        assert_eq!(canvas.render(), "\x1b[39m\x1b[49m \x1b[39m\x1b[49m");
    }

    #[test]
    fn writes_terminal_helpers() {
        let mut output = Vec::new();

        set_cursor_position(&mut output, 12, 34).unwrap();
        hide_cursor(&mut output).unwrap();
        show_cursor(&mut output).unwrap();

        assert_eq!(output, b"\x1b[34;12H\x1b[?25l\x1b[?25h");
    }
}
