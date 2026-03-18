use crate::terminal::Terminal;
use crate::ui::theme::{Color, Theme};
use alacritty_terminal::term::cell::Flags as CellFlags;
use alacritty_terminal::vte::ansi::{Color as TermColor, CursorShape, NamedColor};
use fontdue::{Font, FontSettings};
use std::collections::HashMap;

/// Radius for rounding window corners (transparent masking)
const WINDOW_CORNER_RADIUS: usize = 12;

/// Glyph cache entry
struct GlyphBitmap {
    width: usize,
    height: usize,
    bitmap: Vec<u8>,
    x_offset: i32,
    y_offset: i32,
}

/// Hit zone for a pill button in the title bar
#[derive(Debug, Clone, Copy, Default)]
pub struct PillBounds {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}

impl PillBounds {
    pub fn contains(&self, px: f64, py: f64) -> bool {
        px >= self.x as f64
            && px <= (self.x + self.w) as f64
            && py >= self.y as f64
            && py <= (self.y + self.h) as f64
    }
}

/// For backward compatibility
pub type ThemePillBounds = PillBounds;

/// Software renderer — draws terminal cells into a pixel buffer via fontdue.
pub struct Renderer {
    pub theme: Theme,
    /// Bounds of the theme pill in the title bar (for click detection)
    pub theme_pill: ThemePillBounds,
    /// Bounds of the settings pill in the title bar
    pub settings_pill: PillBounds,
    pub cell_width: usize,
    pub cell_height: usize,
    pub cols: usize,
    pub rows: usize,
    pub title_bar_height: usize,
    /// Padding around the terminal grid (pixels)
    pub pad_left: usize,
    pub pad_right: usize,
    pub pad_top: usize,
    pub pad_bottom: usize,
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

        let pad = 12; // Consistent padding on all sides of the terminal grid

        Self {
            theme,
            theme_pill: ThemePillBounds::default(),
            settings_pill: PillBounds::default(),
            cell_width,
            cell_height,
            cols: 120,
            rows: 35,
            title_bar_height: 36,
            pad_left: pad,
            pad_right: pad,
            pad_top: pad,
            pad_bottom: pad,
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
        let usable_w = (width as usize).saturating_sub(self.pad_left + self.pad_right);
        let usable_h = (height as usize).saturating_sub(self.title_bar_height + self.pad_top + self.pad_bottom);
        self.cols = (usable_w / self.cell_width).max(1);
        self.rows = (usable_h / self.cell_height).max(1);
    }

    /// The x offset where the terminal grid starts (after left padding)
    pub fn grid_x(&self) -> usize {
        self.pad_left
    }

    /// The y offset where the terminal grid starts (after title bar + top padding)
    pub fn grid_y(&self) -> usize {
        self.title_bar_height + self.pad_top
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

    pub fn fill_rect(buf: &mut [u32], stride: usize, x: usize, y: usize, w: usize, h: usize, c: Color) {
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

    /// Fill a rounded rectangle. Radius applies to all four corners.
    fn fill_rounded_rect(
        buf: &mut [u32],
        stride: usize,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        r: usize,
        c: Color,
    ) {
        let packed = pack_color(c);
        let max_y = buf.len() / stride.max(1);
        let r = r.min(w / 2).min(h / 2); // Clamp radius

        for row in 0..h {
            let py = y + row;
            if py >= max_y {
                break;
            }

            // Calculate horizontal inset for this row based on corner radius
            let inset = if row < r {
                // Top corners
                let dy = r - row;
                r - isqrt(r * r - dy * dy)
            } else if row >= h - r {
                // Bottom corners
                let dy = row - (h - r);
                r - isqrt(r * r - dy * dy)
            } else {
                0
            };

            let start = (py * stride + x + inset).min(buf.len());
            let end = (py * stride + x + w - inset).min(py * stride + stride).min(buf.len());
            if start < end {
                buf[start..end].fill(packed);
            }
        }
    }

    /// Draw the outline of a rounded rect (border only, no fill).
    /// Draws by filling the border region between outer and inner rounded rects.
    fn stroke_rounded_rect(
        buf: &mut [u32],
        stride: usize,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        r: usize,
        thickness: usize,
        c: Color,
    ) {
        let packed = pack_color(c);
        let max_y = buf.len() / stride.max(1);
        let r = r.min(w / 2).min(h / 2);
        let inner_r = r.saturating_sub(thickness);

        for row in 0..h {
            let py = y + row;
            if py >= max_y {
                break;
            }

            // Outer inset
            let outer_inset = if row < r {
                let dy = r - row;
                r - isqrt(r * r - dy * dy)
            } else if row >= h - r {
                let dy = row - (h - r);
                r - isqrt(r * r - dy * dy)
            } else {
                0
            };

            // Inner inset (for the hollow interior)
            let in_top_border = row < thickness;
            let in_bottom_border = row >= h - thickness;

            if in_top_border || in_bottom_border {
                // Full row is border
                let start = (py * stride + x + outer_inset).min(buf.len());
                let end = (py * stride + x + w - outer_inset).min(py * stride + stride).min(buf.len());
                if start < end {
                    buf[start..end].fill(packed);
                }
            } else {
                // Left border strip
                let inner_inset = if row < r {
                    let dy = inner_r as isize - (row as isize - thickness as isize);
                    if dy > 0 {
                        inner_r - isqrt(inner_r * inner_r - (dy as usize) * (dy as usize))
                    } else {
                        0
                    }
                } else if row >= h - r {
                    let dy = (row + thickness) as isize - (h - inner_r) as isize;
                    if dy > 0 {
                        inner_r - isqrt(inner_r * inner_r - (dy as usize) * (dy as usize))
                    } else {
                        0
                    }
                } else {
                    0
                };

                let left_start = (py * stride + x + outer_inset).min(buf.len());
                let left_end = (py * stride + x + thickness + inner_inset).min(buf.len());
                if left_start < left_end {
                    buf[left_start..left_end].fill(packed);
                }

                let right_start = (py * stride + x + w - thickness - inner_inset).min(buf.len());
                let right_end = (py * stride + x + w - outer_inset).min(py * stride + stride).min(buf.len());
                if right_start < right_end {
                    buf[right_start..right_end].fill(packed);
                }
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

        // Title text (left side)
        let title = "WindowedClaude";
        let text_y = (self.title_bar_height.saturating_sub(self.cell_height)) / 2;
        self.render_string(buf, stride, 14, text_y, title, self.theme.title_bar_text);

        // Theme selector pill (center-ish, after title)
        let pill_text = self.theme.name;
        let pill_char_count = pill_text.len();
        let pill_text_w = pill_char_count * self.cell_width;
        let pill_pad_h = 6; // Horizontal padding inside pill
        let pill_pad_v = 4; // Vertical padding inside pill
        let pill_w = pill_text_w + pill_pad_h * 2;
        let pill_h = self.cell_height + pill_pad_v * 2;
        let pill_x = 14 + title.len() * self.cell_width + 16; // After title + gap
        let pill_y = (self.title_bar_height.saturating_sub(pill_h)) / 2;

        // Draw pill background (accent color, semi-transparent)
        let pill_bg = Color::rgba(
            self.theme.cursor.r,
            self.theme.cursor.g,
            self.theme.cursor.b,
            50,
        );
        Self::fill_rounded_rect(buf, stride, pill_x, pill_y, pill_w, pill_h, pill_h / 2, pill_bg);

        // Draw pill border
        Self::stroke_rounded_rect(
            buf, stride, pill_x, pill_y, pill_w, pill_h,
            pill_h / 2, 1, self.theme.cursor,
        );

        // Draw pill text
        self.render_string(
            buf, stride,
            pill_x + pill_pad_h, pill_y + pill_pad_v,
            pill_text, self.theme.cursor,
        );

        // Store pill bounds for click detection
        self.theme_pill = ThemePillBounds {
            x: pill_x,
            y: pill_y,
            w: pill_w,
            h: pill_h,
        };

        // Settings pill (after theme pill)
        let settings_text = "Settings";
        let settings_text_w = settings_text.len() * self.cell_width;
        let settings_w = settings_text_w + pill_pad_h * 2;
        let settings_h = pill_h;
        let settings_x = pill_x + pill_w + 10;
        let settings_y = pill_y;

        // Subtle but readable on all themes — use fg color with enough contrast
        let settings_bg = Color::rgb(
            self.theme.title_bar_bg.r.saturating_add(20),
            self.theme.title_bar_bg.g.saturating_add(20),
            self.theme.title_bar_bg.b.saturating_add(20),
        );
        Self::fill_rounded_rect(buf, stride, settings_x, settings_y, settings_w, settings_h, settings_h / 2, settings_bg);

        // Border — use title bar text color for visibility
        Self::stroke_rounded_rect(
            buf, stride, settings_x, settings_y, settings_w, settings_h,
            settings_h / 2, 1, self.theme.title_bar_text,
        );

        // Text — full opacity title bar text so it's always readable
        let settings_text_color = self.theme.title_bar_text;
        self.render_string(
            buf, stride,
            settings_x + pill_pad_h, settings_y + pill_pad_v,
            settings_text, settings_text_color,
        );

        self.settings_pill = PillBounds {
            x: settings_x,
            y: settings_y,
            w: settings_w,
            h: settings_h,
        };

        // Window buttons (right side — traffic light dots)
        let dot_r = 6usize;
        let btn_y = self.title_bar_height / 2;
        let right = stride.saturating_sub(20);

        // Close (red)
        Self::fill_rounded_rect(buf, stride, right.saturating_sub(dot_r), btn_y.saturating_sub(dot_r), dot_r * 2, dot_r * 2, dot_r, Color::rgb(220, 80, 80));
        // Maximize (yellow)
        Self::fill_rounded_rect(buf, stride, right.saturating_sub(dot_r + 28), btn_y.saturating_sub(dot_r), dot_r * 2, dot_r * 2, dot_r, Color::rgb(220, 190, 60));
        // Minimize (green)
        Self::fill_rounded_rect(buf, stride, right.saturating_sub(dot_r + 56), btn_y.saturating_sub(dot_r), dot_r * 2, dot_r * 2, dot_r, Color::rgb(80, 200, 80));
    }

    /// Render a string at pixel coordinates
    pub fn render_string(&mut self, buf: &mut [u32], stride: usize, x: usize, y: usize, text: &str, color: Color) {
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
    /// Render the welcome/shortcut prompt screen
    /// Render the installation progress screen
    pub fn render_install_progress(
        &mut self,
        buf: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        opacity: f32,
        status: &str,
    ) {
        let bg = self.theme.bg_with_opacity(opacity);
        buf.fill(pack_color(bg));

        self.render_title_bar(buf, buf_width, opacity);

        let start_y = self.title_bar_height + (buf_height.saturating_sub(self.title_bar_height)) / 3;

        let lines = [
            "  WindowedClaude",
            "",
            "  Setting up for first use...",
            "",
            &format!("  {}", status),
            "",
            "  This only happens once.",
        ];

        for (i, line) in lines.iter().enumerate() {
            let y = start_y + i * self.cell_height;
            if y + self.cell_height > buf_height {
                break;
            }
            if line.is_empty() {
                continue;
            }
            let color = if line.contains("WindowedClaude") {
                self.theme.cursor
            } else if line.contains("ERROR") {
                self.theme.ansi[1] // Red
            } else if *line == lines[4] {
                self.theme.ansi[2] // Green for current status
            } else {
                self.theme.fg
            };
            self.render_string(buf, buf_width, 0, y, line, color);
        }

        Self::mask_window_corners(buf, buf_width, buf_height);
    }

    /// Render the welcome/shortcut prompt screen
    pub fn render_welcome(
        &mut self,
        buf: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        opacity: f32,
    ) {
        // Clear background
        let bg = self.theme.bg_with_opacity(opacity);
        buf.fill(pack_color(bg));

        // Title bar
        self.render_title_bar(buf, buf_width, opacity);

        // Welcome content — centered vertically
        let lines = [
            "",
            "  Welcome to WindowedClaude",
            "",
            "  Setup complete! Claude Code is ready.",
            "",
            "",
            "  Would you like to create a Desktop shortcut?",
            "",
            "  [Y] Yes, create shortcut",
            "  [N] No thanks",
            "",
            "  (Press Enter for Yes, Escape for No)",
        ];

        let start_y = self.title_bar_height + (buf_height.saturating_sub(self.title_bar_height)) / 4;

        for (i, line) in lines.iter().enumerate() {
            let y = start_y + i * self.cell_height;
            if y + self.cell_height > buf_height {
                break;
            }

            if line.is_empty() {
                continue;
            }

            // Use accent color for the title and option highlights
            let color = if line.contains("Welcome") {
                self.theme.cursor // Amber accent for title
            } else if line.contains("[Y]") || line.contains("[N]") {
                self.theme.ansi[2] // Green for options
            } else if line.contains("Enter") || line.contains("Escape") {
                Color::rgb(
                    self.theme.fg.r / 2 + 40,
                    self.theme.fg.g / 2 + 40,
                    self.theme.fg.b / 2 + 40,
                ) // Dimmed for hint
            } else {
                self.theme.fg
            };

            self.render_string(buf, buf_width, 0, y, line, color);
        }

        Self::mask_window_corners(buf, buf_width, buf_height);
    }

    /// Render a complete terminal frame reading cell data from the terminal
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

        // 3. Padding frame with rounded corners
        //    Strategy: fill the padding zone with border color, then punch out
        //    the terminal grid area as a rounded rect with the bg color.
        let border_color = self.theme.window_border;
        let gx = self.grid_x();
        let gy = self.grid_y();
        let grid_w = self.cols * self.cell_width;
        let grid_h = self.rows * self.cell_height;
        let corner_radius: usize = 10;

        // Fill entire area below title bar with border color
        Self::fill_rect(buf, buf_width, 0, self.title_bar_height, buf_width,
            buf_height.saturating_sub(self.title_bar_height), border_color);

        // Punch out the terminal grid area as a rounded rect with bg color
        // This leaves the border color visible in the padding + rounded corners
        let term_bg = self.theme.bg_with_opacity(opacity);
        if grid_w > 0 && grid_h > 0 {
            Self::fill_rounded_rect(buf, buf_width, gx, gy, grid_w, grid_h, corner_radius, term_bg);
        }

        // 4. Read terminal state and render cells
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

        let gx = self.grid_x();
        let gy = self.grid_y();

        // Render each cell
        for (col, row, ch, fg_color, bg_color, flags, selected) in &cells {
            let px = gx + col * self.cell_width;
            let py = gy + row * self.cell_height;

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
                let px = gx + cx * self.cell_width;
                let py = gy + cy * self.cell_height;
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

        Self::mask_window_corners(buf, buf_width, buf_height);
    }

    // -----------------------------------------------------------------------
    // Settings overlay
    // -----------------------------------------------------------------------

    /// Render the settings overlay on top of everything
    pub fn render_settings_overlay(
        &mut self,
        buf: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        config: &crate::config::Config,
        hover_row: i32,
        click_flash: u8,
    ) {
        // Semi-transparent dark overlay covering whole window
        let overlay = Color::rgba(0, 0, 0, 160);
        for i in 0..buf.len() {
            let bg_packed = buf[i];
            buf[i] = blend(overlay, 160, bg_packed);
        }

        // Centered panel dimensions
        let panel_w = 420usize;
        let panel_h = 320usize;
        let panel_x = (buf_width.saturating_sub(panel_w)) / 2;
        let panel_y = (buf_height.saturating_sub(panel_h)) / 2;
        let panel_r = 14usize;

        // Panel background
        let panel_bg = Color::rgb(
            self.theme.bg.r.saturating_add(15),
            self.theme.bg.g.saturating_add(15),
            self.theme.bg.b.saturating_add(15),
        );
        Self::fill_rounded_rect(buf, buf_width, panel_x, panel_y, panel_w, panel_h, panel_r, panel_bg);

        // Panel border
        Self::stroke_rounded_rect(buf, buf_width, panel_x, panel_y, panel_w, panel_h, panel_r, 1, self.theme.window_border);

        // Header
        let header_y = panel_y + 16;
        self.render_string(buf, buf_width, panel_x + 20, header_y, "Settings", self.theme.cursor);

        // Close X button (top right of panel)
        let close_x = panel_x + panel_w - 30;
        let close_y = panel_y + 12;
        self.render_string(buf, buf_width, close_x, close_y, "X", self.theme.fg);

        // Settings rows
        let row_x = panel_x + 20;
        let value_x = panel_x + 220;
        let mut row_y = header_y + self.cell_height + 20;
        let row_spacing = self.cell_height + 16;

        // Hover/flash colors — fully opaque, blended manually against panel bg
        // Using rgba here causes transparency artifacts on the already-darkened overlay
        let hover_bg = Color::rgb(
            panel_bg.r.saturating_add(self.theme.cursor.r / 10),
            panel_bg.g.saturating_add(self.theme.cursor.g / 10),
            panel_bg.b.saturating_add(self.theme.cursor.b / 10),
        );
        let flash_bg = Color::rgb(
            panel_bg.r.saturating_add(self.theme.cursor.r / 5),
            panel_bg.g.saturating_add(self.theme.cursor.g / 5),
            panel_bg.b.saturating_add(self.theme.cursor.b / 5),
        );

        let mut current_row: i32 = 0;

        // Helper: draw row hover/flash background
        let draw_row_bg = |buf: &mut [u32], row_y: usize, row_idx: i32| {
            if click_flash > 0 && hover_row == row_idx {
                Self::fill_rounded_rect(buf, buf_width, panel_x + 8, row_y.saturating_sub(6), panel_w - 16, row_spacing, 6, flash_bg);
            } else if hover_row == row_idx {
                Self::fill_rounded_rect(buf, buf_width, panel_x + 8, row_y.saturating_sub(6), panel_w - 16, row_spacing, 6, hover_bg);
            }
        };

        // Button style helper colors
        let btn_bg_normal = self.theme.window_border;
        let btn_bg_hover = Color::rgb(
            self.theme.window_border.r.saturating_add(25),
            self.theme.window_border.g.saturating_add(25),
            self.theme.window_border.b.saturating_add(25),
        );

        // Theme row
        draw_row_bg(buf, row_y, current_row);
        self.render_string(buf, buf_width, row_x, row_y, "Theme", self.theme.fg);
        let theme_pill_x = value_x;
        let theme_pill_w = self.theme.name.len() * self.cell_width + 16;
        let theme_pill_h = self.cell_height + 8;
        let theme_pill_y = row_y.saturating_sub(4);
        let pill_alpha = if hover_row == current_row { 80u8 } else { 50u8 };
        Self::fill_rounded_rect(buf, buf_width, theme_pill_x, theme_pill_y, theme_pill_w, theme_pill_h, theme_pill_h / 2, Color::rgba(self.theme.cursor.r, self.theme.cursor.g, self.theme.cursor.b, pill_alpha));
        Self::stroke_rounded_rect(buf, buf_width, theme_pill_x, theme_pill_y, theme_pill_w, theme_pill_h, theme_pill_h / 2, 1, self.theme.cursor);
        self.render_string(buf, buf_width, theme_pill_x + 8, row_y, self.theme.name, self.theme.cursor);
        current_row += 1;
        row_y += row_spacing;

        // Font Size row
        draw_row_bg(buf, row_y, current_row);
        self.render_string(buf, buf_width, row_x, row_y, "Font Size", self.theme.fg);
        let size_str = format!("{:.0}pt", config.font_size);
        self.render_string(buf, buf_width, value_x + 30, row_y, &size_str, self.theme.fg);
        let is_hover = hover_row == current_row;
        let minus_bg = if is_hover { btn_bg_hover } else { btn_bg_normal };
        Self::fill_rounded_rect(buf, buf_width, value_x, row_y.saturating_sub(2), 22, self.cell_height + 4, 4, minus_bg);
        self.render_string(buf, buf_width, value_x + 6, row_y, "-", self.theme.fg);
        let plus_x = value_x + 30 + size_str.len() * self.cell_width + 8;
        Self::fill_rounded_rect(buf, buf_width, plus_x, row_y.saturating_sub(2), 22, self.cell_height + 4, 4, minus_bg);
        self.render_string(buf, buf_width, plus_x + 6, row_y, "+", self.theme.fg);
        current_row += 1;
        row_y += row_spacing;

        // Transparency row
        draw_row_bg(buf, row_y, current_row);
        self.render_string(buf, buf_width, row_x, row_y, "Transparency", self.theme.fg);
        let trans_label = if config.transparent { "On" } else { "Off" };
        let trans_color = if config.transparent { self.theme.ansi[2] } else { self.theme.ansi[1] };
        self.render_string(buf, buf_width, value_x, row_y, trans_label, trans_color);
        current_row += 1;
        row_y += row_spacing;

        // Opacity row (only when transparency is on)
        if config.transparent {
            draw_row_bg(buf, row_y, current_row);
            self.render_string(buf, buf_width, row_x, row_y, "Opacity", self.theme.fg);
            let opacity_str = format!("{:.0}%", config.opacity * 100.0);
            self.render_string(buf, buf_width, value_x + 30, row_y, &opacity_str, self.theme.fg);
            let is_hover = hover_row == current_row;
            let minus_bg = if is_hover { btn_bg_hover } else { btn_bg_normal };
            Self::fill_rounded_rect(buf, buf_width, value_x, row_y.saturating_sub(2), 22, self.cell_height + 4, 4, minus_bg);
            self.render_string(buf, buf_width, value_x + 6, row_y, "-", self.theme.fg);
            let plus_x = value_x + 30 + opacity_str.len() * self.cell_width + 8;
            Self::fill_rounded_rect(buf, buf_width, plus_x, row_y.saturating_sub(2), 22, self.cell_height + 4, 4, minus_bg);
            self.render_string(buf, buf_width, plus_x + 6, row_y, "+", self.theme.fg);
            current_row += 1;
            row_y += row_spacing;
        }

        // Reinstall Shortcuts button
        let btn_text = "Reinstall Shortcuts";
        let btn_w = btn_text.len() * self.cell_width + 24;
        let btn_h = self.cell_height + 12;
        let btn_x = row_x;
        let btn_y = row_y;
        let is_btn_hover = hover_row == current_row;
        let is_btn_flash = click_flash > 0 && hover_row == current_row;
        let btn_fill = if is_btn_flash {
            self.theme.cursor
        } else if is_btn_hover {
            btn_bg_hover
        } else {
            btn_bg_normal
        };
        let btn_text_color = if is_btn_flash {
            self.theme.bg
        } else {
            self.theme.fg
        };
        Self::fill_rounded_rect(buf, buf_width, btn_x, btn_y, btn_w, btn_h, 6, btn_fill);
        Self::stroke_rounded_rect(buf, buf_width, btn_x, btn_y, btn_w, btn_h, 6, 1, self.theme.fg);
        self.render_string(buf, buf_width, btn_x + 12, btn_y + 6, btn_text, btn_text_color);

        // Hint at bottom
        let hint = "Press Escape to close";
        let hint_y = panel_y + panel_h - self.cell_height - 12;
        let hint_color = Color::rgba(self.theme.fg.r, self.theme.fg.g, self.theme.fg.b, 100);
        self.render_string(buf, buf_width, panel_x + 20, hint_y, hint, hint_color);
    }

    /// Get settings panel bounds for hit testing
    pub fn settings_panel_bounds(&self, buf_width: usize, buf_height: usize) -> (usize, usize, usize, usize) {
        let panel_w = 420usize;
        let panel_h = 320usize;
        let panel_x = (buf_width.saturating_sub(panel_w)) / 2;
        let panel_y = (buf_height.saturating_sub(panel_h)) / 2;
        (panel_x, panel_y, panel_w, panel_h)
    }

    // -----------------------------------------------------------------------
    // Window corner masking — "cookie cutter" transparent corners
    // -----------------------------------------------------------------------

    /// Mask off pixels outside the rounded window boundary to transparent.
    /// Called after each full render pass to create rounded window corners.
    pub fn mask_window_corners(buf: &mut [u32], width: usize, height: usize) {
        let r = WINDOW_CORNER_RADIUS;
        if r == 0 || width == 0 || height == 0 {
            return;
        }
        let r = r.min(width / 2).min(height / 2);

        for row in 0..r {
            let dy = r - row;
            let dx = r - isqrt(r * r - dy * dy);
            // Top-left corner
            let start = row * width;
            for col in 0..dx {
                if start + col < buf.len() {
                    buf[start + col] = 0x00000000;
                }
            }
            // Top-right corner
            for col in (width - dx)..width {
                if start + col < buf.len() {
                    buf[start + col] = 0x00000000;
                }
            }
        }

        for row in (height - r)..height {
            let dy = row - (height - r);
            let dx = r - isqrt(r * r - dy * dy);
            let start = row * width;
            // Bottom-left corner
            for col in 0..dx {
                if start + col < buf.len() {
                    buf[start + col] = 0x00000000;
                }
            }
            // Bottom-right corner
            for col in (width - dx)..width {
                if start + col < buf.len() {
                    buf[start + col] = 0x00000000;
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Color utilities
// ---------------------------------------------------------------------------

fn pack_color(c: Color) -> u32 {
    // Always include alpha in high byte — required since window uses with_transparent(true)
    let alpha = c.a as u32;
    if c.a == 255 {
        (alpha << 24) | ((c.r as u32) << 16) | ((c.g as u32) << 8) | (c.b as u32)
    } else {
        let a = c.a as f32 / 255.0;
        let r = (c.r as f32 * a) as u32;
        let g = (c.g as f32 * a) as u32;
        let b = (c.b as f32 * a) as u32;
        (alpha << 24) | (r << 16) | (g << 8) | b
    }
}

fn blend(fg: Color, alpha: u8, bg_packed: u32) -> u32 {
    let a = alpha as f32 / 255.0;
    let inv = 1.0 - a;
    let bg_a = ((bg_packed >> 24) & 0xFF) as f32;
    let br = ((bg_packed >> 16) & 0xFF) as f32;
    let bg = ((bg_packed >> 8) & 0xFF) as f32;
    let bb = (bg_packed & 0xFF) as f32;
    let r = (fg.r as f32 * a + br * inv) as u32;
    let g = (fg.g as f32 * a + bg * inv) as u32;
    let b = (fg.b as f32 * a + bb * inv) as u32;
    // Preserve or boost alpha — blending over a pixel should keep it opaque
    let out_a = (bg_a + (255.0 - bg_a) * a) as u32;
    (out_a.min(255) << 24) | (r.min(255) << 16) | (g.min(255) << 8) | b.min(255)
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

/// Integer square root (floor)
fn isqrt(n: usize) -> usize {
    if n == 0 {
        return 0;
    }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}
