use crate::terminal::Terminal;
use crate::ui::theme::{Color, Theme};
use alacritty_terminal::term::cell::Flags as CellFlags;
use alacritty_terminal::vte::ansi::{Color as TermColor, CursorShape, NamedColor};
use fontdue::{Font, FontSettings};
use std::collections::HashMap;

/// Glyph cache entry
struct GlyphBitmap {
    width: usize,
    height: usize,
    bitmap: Vec<u8>,
    x_offset: i32,
    y_offset: i32,
}

/// Software renderer — draws terminal cells into a pixel buffer via fontdue.
pub struct Renderer {
    pub theme: Theme,
    pub cell_width: usize,
    pub cell_height: usize,
    pub cols: usize,
    pub rows: usize,
    pub title_bar_height: usize,
    font: Font,
    font_bold: Font,
    font_size: f32,
    ascent: usize,
    glyph_cache: HashMap<(char, bool), GlyphBitmap>,
}

impl Renderer {
    pub fn new(theme: Theme, font_size: f32) -> Self {
        let font = Self::load_font(false);
        let font_bold = Self::load_font(true);

        let metrics = font.horizontal_line_metrics(font_size).unwrap_or(
            fontdue::LineMetrics {
                ascent: font_size * 0.8,
                descent: font_size * -0.2,
                line_gap: 0.0,
                new_line_size: font_size,
            },
        );

        let (char_metrics, _) = font.rasterize('M', font_size);
        let cell_width = (char_metrics.advance_width.ceil() as usize).max(1);
        let cell_height = ((metrics.ascent - metrics.descent + metrics.line_gap).ceil() as usize).max(1);
        let ascent = metrics.ascent.ceil() as usize;

        Self {
            theme,
            cell_width,
            cell_height,
            cols: 120,
            rows: 35,
            title_bar_height: 36,
            font,
            font_bold,
            font_size,
            ascent,
            glyph_cache: HashMap::new(),
        }
    }

    fn load_font(bold: bool) -> Font {
        let settings = FontSettings::default();
        // Both use regular for now — add JetBrainsMono-Bold.ttf later
        let _ = bold;
        Font::from_bytes(
            include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf") as &[u8],
            settings,
        )
        .expect("Failed to load embedded font")
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let term_h = (height as usize).saturating_sub(self.title_bar_height);
        self.cols = (width as usize / self.cell_width).max(1);
        self.rows = (term_h / self.cell_height).max(1);
    }

    /// Get or rasterize a glyph
    fn rasterize_glyph(&mut self, ch: char, bold: bool) {
        let key = (ch, bold);
        if self.glyph_cache.contains_key(&key) {
            return;
        }
        let font = if bold { &self.font_bold } else { &self.font };
        let (metrics, bitmap) = font.rasterize(ch, self.font_size);
        self.glyph_cache.insert(key, GlyphBitmap {
            width: metrics.width,
            height: metrics.height,
            bitmap,
            x_offset: metrics.xmin,
            y_offset: metrics.ymin,
        });
    }

    // -----------------------------------------------------------------------
    // Color mapping: alacritty Color → theme Color
    // -----------------------------------------------------------------------

    fn resolve_fg(&self, color: &TermColor) -> Color {
        match color {
            TermColor::Named(n) => self.named_color(n),
            TermColor::Spec(rgb) => Color::rgb(rgb.r, rgb.g, rgb.b),
            TermColor::Indexed(i) if *i < 16 => self.theme.ansi[*i as usize],
            TermColor::Indexed(i) => ansi256_to_rgb(*i),
        }
    }

    fn resolve_bg(&self, color: &TermColor) -> Color {
        match color {
            TermColor::Named(NamedColor::Background) => self.theme.bg,
            TermColor::Named(n) => self.named_color(n),
            TermColor::Spec(rgb) => Color::rgb(rgb.r, rgb.g, rgb.b),
            TermColor::Indexed(i) if *i < 16 => self.theme.ansi[*i as usize],
            TermColor::Indexed(i) => ansi256_to_rgb(*i),
        }
    }

    fn named_color(&self, named: &NamedColor) -> Color {
        match named {
            NamedColor::Black => self.theme.ansi[0],
            NamedColor::Red => self.theme.ansi[1],
            NamedColor::Green => self.theme.ansi[2],
            NamedColor::Yellow => self.theme.ansi[3],
            NamedColor::Blue => self.theme.ansi[4],
            NamedColor::Magenta => self.theme.ansi[5],
            NamedColor::Cyan => self.theme.ansi[6],
            NamedColor::White => self.theme.ansi[7],
            NamedColor::BrightBlack => self.theme.ansi[8],
            NamedColor::BrightRed => self.theme.ansi[9],
            NamedColor::BrightGreen => self.theme.ansi[10],
            NamedColor::BrightYellow => self.theme.ansi[11],
            NamedColor::BrightBlue => self.theme.ansi[12],
            NamedColor::BrightMagenta => self.theme.ansi[13],
            NamedColor::BrightCyan => self.theme.ansi[14],
            NamedColor::BrightWhite => self.theme.ansi[15],
            NamedColor::Foreground => self.theme.fg,
            NamedColor::Background => self.theme.bg,
            NamedColor::Cursor => self.theme.cursor,
            _ => self.theme.fg, // Dim variants → fg
        }
    }

    // -----------------------------------------------------------------------
    // Drawing primitives
    // -----------------------------------------------------------------------

    fn fill_rect(buf: &mut [u32], stride: usize, x: usize, y: usize, w: usize, h: usize, c: Color) {
        let packed = pack_color(c);
        let max_y = buf.len() / stride.max(1);
        for row in y..(y + h).min(max_y) {
            let start = row * stride + x;
            let end = (start + w).min(row * stride + stride).min(buf.len());
            if start < buf.len() {
                buf[start..end].fill(packed);
            }
        }
    }

    fn draw_glyph_at(
        buf: &mut [u32],
        stride: usize,
        glyph: &GlyphBitmap,
        px: usize,
        py: usize,
        ascent: usize,
        fg: Color,
    ) {
        let gx = px as i32 + glyph.x_offset;
        let gy = py as i32 + ascent as i32 - glyph.y_offset - glyph.height as i32;
        let max_y = buf.len() / stride.max(1);

        for row in 0..glyph.height {
            let y = gy + row as i32;
            if y < 0 { continue; }
            let y = y as usize;
            if y >= max_y { break; }

            for col in 0..glyph.width {
                let x = gx + col as i32;
                if x < 0 { continue; }
                let x = x as usize;
                if x >= stride { break; }

                let alpha = glyph.bitmap[row * glyph.width + col];
                if alpha == 0 { continue; }

                let idx = y * stride + x;
                if idx >= buf.len() { continue; }

                if alpha == 255 {
                    buf[idx] = pack_color(fg);
                } else {
                    buf[idx] = blend(fg, alpha, buf[idx]);
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Title bar
    // -----------------------------------------------------------------------

    fn render_title_bar(&mut self, buf: &mut [u32], stride: usize, opacity: f32) {
        let bg = self.theme.title_bar_bg_with_opacity(opacity);
        Self::fill_rect(buf, stride, 0, 0, stride, self.title_bar_height, bg);

        // Title text
        let title = format!("WindowedClaude  —  {}", self.theme.name);
        let text_y = (self.title_bar_height.saturating_sub(self.cell_height)) / 2;
        self.render_string(buf, stride, 14, text_y, &title, self.theme.title_bar_text);

        // Window buttons (right side)
        let dot_r = 6usize;
        let btn_y = self.title_bar_height / 2;
        let right = stride.saturating_sub(20);

        // Close (red dot)
        Self::fill_rect(buf, stride, right.saturating_sub(dot_r), btn_y.saturating_sub(dot_r), dot_r * 2, dot_r * 2, Color::rgb(220, 80, 80));
        // Maximize (yellow dot)
        Self::fill_rect(buf, stride, right.saturating_sub(dot_r + 28), btn_y.saturating_sub(dot_r), dot_r * 2, dot_r * 2, Color::rgb(220, 190, 60));
        // Minimize (green dot)
        Self::fill_rect(buf, stride, right.saturating_sub(dot_r + 56), btn_y.saturating_sub(dot_r), dot_r * 2, dot_r * 2, Color::rgb(80, 200, 80));
    }

    /// Render a string at pixel coordinates
    fn render_string(&mut self, buf: &mut [u32], stride: usize, x: usize, y: usize, text: &str, color: Color) {
        let mut cx = x;
        for ch in text.chars() {
            self.rasterize_glyph(ch, false);
            if let Some(glyph) = self.glyph_cache.get(&(ch, false)) {
                // Clone bitmap data to avoid borrow conflict
                let bmp = GlyphBitmap {
                    width: glyph.width,
                    height: glyph.height,
                    bitmap: glyph.bitmap.clone(),
                    x_offset: glyph.x_offset,
                    y_offset: glyph.y_offset,
                };
                Self::draw_glyph_at(buf, stride, &bmp, cx, y, self.ascent, color);
            }
            cx += self.cell_width;
        }
    }

    // -----------------------------------------------------------------------
    // Full frame render — reads from alacritty Term
    // -----------------------------------------------------------------------

    /// Render a complete frame reading cell data from the terminal
    pub fn render_frame(
        &mut self,
        buf: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        opacity: f32,
        terminal: &Terminal,
    ) {
        // 1. Clear background
        let bg = self.theme.bg_with_opacity(opacity);
        buf.fill(pack_color(bg));

        // 2. Title bar
        self.render_title_bar(buf, buf_width, opacity);

        // 3. Read terminal state and render cells
        let term = terminal.term.lock().unwrap();
        let content = term.renderable_content();

        let cursor_point = content.cursor.point;
        let cursor_shape = content.cursor.shape;

        // Get selection range for highlight rendering
        let selection = content.selection;

        // Pre-collect cell data to release the lock sooner
        let mut cells: Vec<(usize, usize, char, TermColor, TermColor, CellFlags, bool)> = Vec::new();

        for indexed in content.display_iter {
            let col = indexed.point.column.0;
            let row = indexed.point.line.0 as usize;
            let cell = &indexed.cell;

            // Check if this cell is within the selection
            let selected = selection.as_ref().map_or(false, |sel| {
                sel.contains(indexed.point)
            });

            // Skip cells outside our grid
            if col >= self.cols || row >= self.rows {
                continue;
            }

            cells.push((col, row, cell.c, cell.fg, cell.bg, cell.flags, selected));
        }

        drop(term); // Release the mutex before rendering

        // Render each cell
        for (col, row, ch, fg_color, bg_color, flags, selected) in &cells {
            let px = col * self.cell_width;
            let py = self.title_bar_height + row * self.cell_height;

            if py + self.cell_height > buf_height {
                continue;
            }

            // Resolve colors through theme
            let fg = self.resolve_fg(fg_color);
            let cell_bg = self.resolve_bg(bg_color);

            // Handle reverse video
            let (fg, cell_bg) = if flags.contains(CellFlags::INVERSE) {
                (cell_bg, fg)
            } else {
                (fg, cell_bg)
            };

            // Selection highlight: use theme selection colors
            let (fg, cell_bg) = if *selected {
                (self.theme.selection_fg, self.theme.selection_bg)
            } else {
                (fg, cell_bg)
            };

            // Draw cell background (skip if same as terminal bg to save work)
            if cell_bg != self.theme.bg {
                let cell_bg_opacity = if opacity < 1.0 {
                    Color::rgba(cell_bg.r, cell_bg.g, cell_bg.b, (opacity * 255.0) as u8)
                } else {
                    cell_bg
                };
                Self::fill_rect(buf, buf_width, px, py, self.cell_width, self.cell_height, cell_bg_opacity);
            }

            // Draw glyph
            if *ch != ' ' && *ch != '\0' {
                let bold = flags.contains(CellFlags::BOLD);
                self.rasterize_glyph(*ch, bold);
                if let Some(glyph) = self.glyph_cache.get(&(*ch, bold)) {
                    let bmp = GlyphBitmap {
                        width: glyph.width,
                        height: glyph.height,
                        bitmap: glyph.bitmap.clone(),
                        x_offset: glyph.x_offset,
                        y_offset: glyph.y_offset,
                    };
                    Self::draw_glyph_at(buf, buf_width, &bmp, px, py, self.ascent, fg);
                }
            }

            // Underline
            if flags.contains(CellFlags::UNDERLINE) {
                let underline_y = py + self.cell_height - 2;
                Self::fill_rect(buf, buf_width, px, underline_y, self.cell_width, 1, fg);
            }

            // Strikethrough
            if flags.contains(CellFlags::STRIKEOUT) {
                let strike_y = py + self.cell_height / 2;
                Self::fill_rect(buf, buf_width, px, strike_y, self.cell_width, 1, fg);
            }
        }

        // 4. Cursor
        if cursor_shape != CursorShape::Hidden {
            let cx = cursor_point.column.0;
            let cy = cursor_point.line.0 as usize;
            if cx < self.cols && cy < self.rows {
                let px = cx * self.cell_width;
                let py = self.title_bar_height + cy * self.cell_height;
                let cursor_c = Color::rgba(
                    self.theme.cursor.r,
                    self.theme.cursor.g,
                    self.theme.cursor.b,
                    160,
                );
                match cursor_shape {
                    CursorShape::Block => {
                        Self::fill_rect(buf, buf_width, px, py, self.cell_width, self.cell_height, cursor_c);
                    }
                    CursorShape::Beam => {
                        Self::fill_rect(buf, buf_width, px, py, 2, self.cell_height, self.theme.cursor);
                    }
                    CursorShape::Underline => {
                        Self::fill_rect(buf, buf_width, px, py + self.cell_height - 2, self.cell_width, 2, self.theme.cursor);
                    }
                    _ => {}
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Color utilities
// ---------------------------------------------------------------------------

fn pack_color(c: Color) -> u32 {
    if c.a == 255 {
        ((c.r as u32) << 16) | ((c.g as u32) << 8) | (c.b as u32)
    } else {
        let a = c.a as f32 / 255.0;
        let r = (c.r as f32 * a) as u32;
        let g = (c.g as f32 * a) as u32;
        let b = (c.b as f32 * a) as u32;
        (r << 16) | (g << 8) | b
    }
}

fn blend(fg: Color, alpha: u8, bg_packed: u32) -> u32 {
    let a = alpha as f32 / 255.0;
    let inv = 1.0 - a;
    let br = ((bg_packed >> 16) & 0xFF) as f32;
    let bg = ((bg_packed >> 8) & 0xFF) as f32;
    let bb = (bg_packed & 0xFF) as f32;
    let r = (fg.r as f32 * a + br * inv) as u32;
    let g = (fg.g as f32 * a + bg * inv) as u32;
    let b = (fg.b as f32 * a + bb * inv) as u32;
    (r.min(255) << 16) | (g.min(255) << 8) | b.min(255)
}

/// Convert ANSI 256-color index (16-255) to RGB
fn ansi256_to_rgb(idx: u8) -> Color {
    if idx < 16 {
        // Should be handled by caller via theme, but fallback
        return Color::rgb(128, 128, 128);
    }
    if idx < 232 {
        // 6x6x6 color cube: indices 16-231
        let idx = idx - 16;
        let r = (idx / 36) * 51;
        let g = ((idx % 36) / 6) * 51;
        let b = (idx % 6) * 51;
        Color::rgb(r, g, b)
    } else {
        // Grayscale ramp: indices 232-255
        let v = 8 + (idx - 232) * 10;
        Color::rgb(v, v, v)
    }
}
