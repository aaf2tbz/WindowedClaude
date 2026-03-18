/// WindowedClaude theme system
///
/// Multiple built-in themes with full ANSI 16-color palettes,
/// transparency support, and runtime switching.

/// All available theme IDs — order matches the cycle toggle
pub const THEME_IDS: &[&str] = &[
    "claude-dark",
    "claude-light",
    "midnight",
    "solarized-dark",
    "dracula",
    "nord",
    "monokai",
    "gruvbox",
    "developer",
];

/// Look up a theme by its ID string. Returns Claude Dark if not found.
pub fn theme_by_id(id: &str) -> &'static Theme {
    match id {
        "claude-dark" => &CLAUDE_DARK,
        "claude-light" => &CLAUDE_LIGHT,
        "midnight" => &MIDNIGHT,
        "solarized-dark" => &SOLARIZED_DARK,
        "dracula" => &DRACULA,
        "nord" => &NORD,
        "monokai" => &MONOKAI,
        "gruvbox" => &GRUVBOX,
        "developer" => &DEVELOPER,
        _ => &CLAUDE_DARK,
    }
}

/// Cycle to the next theme ID after the given one
pub fn next_theme_id(current: &str) -> &'static str {
    let idx = THEME_IDS.iter().position(|&id| id == current).unwrap_or(0);
    THEME_IDS[(idx + 1) % THEME_IDS.len()]
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub id: &'static str,
    pub name: &'static str,

    // Window chrome
    pub title_bar_bg: Color,
    pub title_bar_text: Color,
    pub window_border: Color,

    // Terminal
    pub bg: Color,
    pub fg: Color,
    pub cursor: Color,
    pub selection_bg: Color,
    pub selection_fg: Color,

    // ANSI 16 colors: [black, red, green, yellow, blue, magenta, cyan, white] x2
    pub ansi: [Color; 16],

    /// When true, render all UI text (title bar, pills, overlays) in bold
    pub bold_ui: bool,
}

impl Theme {
    /// Look up an ANSI color by index (0-15)
    pub fn ansi_color(&self, index: u8) -> Color {
        self.ansi[index.min(15) as usize]
    }

    /// Get the background color with an opacity override applied
    pub fn bg_with_opacity(&self, opacity: f32) -> Color {
        Color::rgba(self.bg.r, self.bg.g, self.bg.b, (opacity * 255.0) as u8)
    }

    /// Get the title bar bg with opacity applied
    pub fn title_bar_bg_with_opacity(&self, opacity: f32) -> Color {
        Color::rgba(
            self.title_bar_bg.r,
            self.title_bar_bg.g,
            self.title_bar_bg.b,
            (opacity * 255.0) as u8,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Convert to normalized f32 array for GL uniforms
    pub fn to_f32(&self) -> [f32; 4] {
        [
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
            self.a as f32 / 255.0,
        ]
    }

    /// Blend this color over a background (premultiplied alpha composite)
    pub fn over(&self, bg: Color) -> Color {
        let sa = self.a as f32 / 255.0;
        let da = bg.a as f32 / 255.0;
        let out_a = sa + da * (1.0 - sa);
        if out_a == 0.0 {
            return Color::rgba(0, 0, 0, 0);
        }
        let blend = |s: u8, d: u8| -> u8 {
            ((s as f32 * sa + d as f32 * da * (1.0 - sa)) / out_a) as u8
        };
        Color::rgba(
            blend(self.r, bg.r),
            blend(self.g, bg.g),
            blend(self.b, bg.b),
            (out_a * 255.0) as u8,
        )
    }
}

// ---------------------------------------------------------------------------
// Built-in themes
// ---------------------------------------------------------------------------

pub const CLAUDE_DARK: Theme = Theme {
    id: "claude-dark",
    name: "Claude Dark",
    title_bar_bg: Color::rgb(24, 24, 28),
    title_bar_text: Color::rgb(200, 200, 210),
    window_border: Color::rgb(45, 45, 55),
    bg: Color::rgb(13, 13, 17),
    fg: Color::rgb(220, 220, 230),
    cursor: Color::rgb(209, 142, 97),
    selection_bg: Color::rgb(55, 40, 30),
    selection_fg: Color::rgb(255, 255, 255),
    ansi: [
        Color::rgb(30, 30, 36),     // black
        Color::rgb(220, 95, 95),    // red
        Color::rgb(130, 200, 130),  // green
        Color::rgb(220, 190, 110),  // yellow
        Color::rgb(110, 150, 220),  // blue
        Color::rgb(180, 130, 200),  // magenta
        Color::rgb(110, 200, 200),  // cyan
        Color::rgb(200, 200, 210),  // white
        Color::rgb(80, 80, 90),     // bright black
        Color::rgb(240, 120, 120),  // bright red
        Color::rgb(160, 230, 160),  // bright green
        Color::rgb(240, 210, 140),  // bright yellow
        Color::rgb(140, 180, 240),  // bright blue
        Color::rgb(210, 160, 230),  // bright magenta
        Color::rgb(140, 230, 230),  // bright cyan
        Color::rgb(240, 240, 250),  // bright white
    ],
    bold_ui: false,
};

pub const CLAUDE_LIGHT: Theme = Theme {
    id: "claude-light",
    name: "Claude Light",
    title_bar_bg: Color::rgb(245, 243, 240),
    title_bar_text: Color::rgb(50, 50, 55),
    window_border: Color::rgb(210, 205, 200),
    bg: Color::rgb(252, 250, 248),
    fg: Color::rgb(40, 40, 45),
    cursor: Color::rgb(180, 100, 60),
    selection_bg: Color::rgb(220, 200, 185),
    selection_fg: Color::rgb(40, 40, 45),
    ansi: [
        Color::rgb(40, 40, 45),
        Color::rgb(185, 55, 55),
        Color::rgb(40, 140, 60),
        Color::rgb(160, 120, 20),
        Color::rgb(50, 100, 185),
        Color::rgb(140, 70, 165),
        Color::rgb(30, 140, 150),
        Color::rgb(230, 228, 225),
        Color::rgb(120, 120, 130),
        Color::rgb(210, 80, 80),
        Color::rgb(60, 170, 80),
        Color::rgb(190, 150, 40),
        Color::rgb(80, 130, 210),
        Color::rgb(170, 100, 195),
        Color::rgb(50, 170, 180),
        Color::rgb(252, 250, 248),
    ],
    bold_ui: false,
};

pub const MIDNIGHT: Theme = Theme {
    id: "midnight",
    name: "Midnight",
    title_bar_bg: Color::rgb(8, 8, 16),
    title_bar_text: Color::rgb(150, 170, 210),
    window_border: Color::rgb(30, 35, 60),
    bg: Color::rgb(5, 5, 12),
    fg: Color::rgb(190, 205, 235),
    cursor: Color::rgb(100, 140, 255),
    selection_bg: Color::rgb(25, 30, 55),
    selection_fg: Color::rgb(230, 235, 255),
    ansi: [
        Color::rgb(20, 20, 35),     // black — boosted from 15 for contrast
        Color::rgb(255, 85, 110),
        Color::rgb(80, 220, 140),
        Color::rgb(240, 200, 80),
        Color::rgb(80, 140, 255),
        Color::rgb(190, 120, 255),
        Color::rgb(80, 210, 230),
        Color::rgb(190, 200, 225),   // white — brighter
        Color::rgb(65, 70, 100),     // bright black — more visible
        Color::rgb(255, 120, 140),
        Color::rgb(120, 240, 170),
        Color::rgb(255, 220, 110),
        Color::rgb(120, 170, 255),
        Color::rgb(210, 150, 255),
        Color::rgb(120, 230, 245),
        Color::rgb(225, 230, 248),
    ],
    bold_ui: false,
};

pub const SOLARIZED_DARK: Theme = Theme {
    id: "solarized-dark",
    name: "Solarized Dark",
    title_bar_bg: Color::rgb(0, 34, 43),
    title_bar_text: Color::rgb(131, 148, 150),
    window_border: Color::rgb(7, 54, 66),
    bg: Color::rgb(0, 43, 54),
    fg: Color::rgb(131, 148, 150),
    cursor: Color::rgb(203, 75, 22),
    selection_bg: Color::rgb(7, 54, 66),
    selection_fg: Color::rgb(147, 161, 161),
    ansi: [
        Color::rgb(7, 54, 66),       // base02  (black)
        Color::rgb(220, 50, 47),     // red
        Color::rgb(133, 153, 0),     // green
        Color::rgb(181, 137, 0),     // yellow
        Color::rgb(38, 139, 210),    // blue
        Color::rgb(211, 54, 130),    // magenta
        Color::rgb(42, 161, 152),    // cyan
        Color::rgb(238, 232, 213),   // base2   (white)
        Color::rgb(0, 43, 54),       // base03  (bright black) — canonical
        Color::rgb(203, 75, 22),     // orange  (bright red) — canonical
        Color::rgb(88, 110, 117),    // base01  (bright green) — canonical
        Color::rgb(101, 123, 131),   // base00  (bright yellow) — canonical
        Color::rgb(131, 148, 150),   // base0   (bright blue) — canonical
        Color::rgb(108, 113, 196),   // violet  (bright magenta) — canonical
        Color::rgb(147, 161, 161),   // base1   (bright cyan) — canonical
        Color::rgb(253, 246, 227),   // base3   (bright white) — canonical
    ],
    bold_ui: false,
};

pub const DRACULA: Theme = Theme {
    id: "dracula",
    name: "Dracula",
    title_bar_bg: Color::rgb(34, 33, 46),
    title_bar_text: Color::rgb(248, 248, 242),
    window_border: Color::rgb(68, 71, 90),
    bg: Color::rgb(40, 42, 54),
    fg: Color::rgb(248, 248, 242),
    cursor: Color::rgb(248, 248, 242),
    selection_bg: Color::rgb(68, 71, 90),
    selection_fg: Color::rgb(248, 248, 242),
    ansi: [
        Color::rgb(33, 34, 44),       // black
        Color::rgb(255, 85, 85),      // red
        Color::rgb(80, 250, 123),     // green
        Color::rgb(241, 250, 140),    // yellow
        Color::rgb(189, 147, 249),    // blue — fixed: was using comment color (98,114,164)
        Color::rgb(255, 121, 198),    // magenta
        Color::rgb(139, 233, 253),    // cyan
        Color::rgb(248, 248, 242),    // white
        Color::rgb(68, 71, 90),       // bright black — fixed: was comment, now currentLine
        Color::rgb(255, 110, 110),
        Color::rgb(105, 255, 148),
        Color::rgb(246, 255, 165),
        Color::rgb(202, 169, 255),    // bright blue — canonical purple bright
        Color::rgb(255, 146, 213),
        Color::rgb(164, 248, 255),
        Color::rgb(255, 255, 255),
    ],
    bold_ui: false,
};

pub const NORD: Theme = Theme {
    id: "nord",
    name: "Nord",
    title_bar_bg: Color::rgb(36, 40, 49),
    title_bar_text: Color::rgb(216, 222, 233),
    window_border: Color::rgb(59, 66, 82),
    bg: Color::rgb(46, 52, 64),
    fg: Color::rgb(216, 222, 233),
    cursor: Color::rgb(216, 222, 233),
    selection_bg: Color::rgb(67, 76, 94),
    selection_fg: Color::rgb(236, 239, 244),
    ansi: [
        Color::rgb(59, 66, 82),       // black (nord3)
        Color::rgb(191, 97, 106),     // red (nord11)
        Color::rgb(163, 190, 140),    // green (nord14)
        Color::rgb(235, 203, 139),    // yellow (nord13)
        Color::rgb(129, 161, 193),    // blue (nord9)
        Color::rgb(180, 142, 173),    // magenta (nord15)
        Color::rgb(136, 192, 208),    // cyan (nord7)
        Color::rgb(229, 233, 240),    // white (nord5)
        Color::rgb(76, 86, 106),      // bright black (nord3 bright) — differentiated
        Color::rgb(210, 120, 130),    // bright red — lighter
        Color::rgb(185, 210, 165),    // bright green — lighter
        Color::rgb(245, 220, 165),    // bright yellow — lighter
        Color::rgb(155, 185, 215),    // bright blue — lighter
        Color::rgb(200, 165, 195),    // bright magenta — lighter
        Color::rgb(160, 210, 225),    // bright cyan — lighter
        Color::rgb(236, 239, 244),    // bright white (nord6)
    ],
    bold_ui: false,
};

pub const MONOKAI: Theme = Theme {
    id: "monokai",
    name: "Monokai Pro",
    title_bar_bg: Color::rgb(35, 33, 31),
    title_bar_text: Color::rgb(252, 252, 250),
    window_border: Color::rgb(55, 53, 50),
    bg: Color::rgb(45, 43, 40),
    fg: Color::rgb(252, 252, 250),
    cursor: Color::rgb(252, 252, 250),
    selection_bg: Color::rgb(87, 82, 74),
    selection_fg: Color::rgb(252, 252, 250),
    ansi: [
        Color::rgb(55, 53, 50),       // black
        Color::rgb(255, 97, 136),     // red
        Color::rgb(169, 220, 118),    // green
        Color::rgb(255, 216, 102),    // yellow
        Color::rgb(147, 165, 255),    // blue — fixed: was identical to cyan
        Color::rgb(171, 157, 242),    // magenta
        Color::rgb(120, 220, 232),    // cyan
        Color::rgb(252, 252, 250),    // white
        Color::rgb(114, 109, 103),    // bright black
        Color::rgb(255, 130, 160),    // bright red
        Color::rgb(190, 235, 145),    // bright green
        Color::rgb(255, 228, 140),    // bright yellow
        Color::rgb(170, 185, 255),    // bright blue — distinct
        Color::rgb(195, 182, 250),    // bright magenta
        Color::rgb(150, 235, 245),    // bright cyan
        Color::rgb(255, 255, 255),    // bright white
    ],
    bold_ui: false,
};

pub const GRUVBOX: Theme = Theme {
    id: "gruvbox",
    name: "Gruvbox Dark",
    title_bar_bg: Color::rgb(29, 32, 33),
    title_bar_text: Color::rgb(235, 219, 178),
    window_border: Color::rgb(60, 56, 54),
    bg: Color::rgb(40, 40, 40),
    fg: Color::rgb(235, 219, 178),
    cursor: Color::rgb(254, 128, 25),
    selection_bg: Color::rgb(69, 65, 57),
    selection_fg: Color::rgb(235, 219, 178),
    ansi: [
        Color::rgb(40, 40, 40),       // bg0
        Color::rgb(204, 36, 29),      // red
        Color::rgb(152, 151, 26),     // green
        Color::rgb(215, 153, 33),     // yellow
        Color::rgb(69, 133, 136),     // blue
        Color::rgb(177, 98, 134),     // magenta
        Color::rgb(104, 157, 106),    // cyan
        Color::rgb(168, 153, 132),    // fg4 (white)
        Color::rgb(124, 111, 100),    // bright black — refined to bg4
        Color::rgb(251, 73, 52),      // bright red
        Color::rgb(184, 187, 38),     // bright green
        Color::rgb(250, 189, 47),     // bright yellow
        Color::rgb(131, 165, 152),    // bright blue
        Color::rgb(211, 134, 155),    // bright magenta
        Color::rgb(142, 192, 124),    // bright cyan
        Color::rgb(235, 219, 178),    // bright white (fg1)
    ],
    bold_ui: false,
};

// ---------------------------------------------------------------------------
// Developer Theme — obsidian + electric. Bold. Clean. Surgical.
// ---------------------------------------------------------------------------
//
// Design philosophy:
//   - True black background (#0A0A0A) — no grey, no warmth, just void
//   - Electric cyan accent (#00E5FF) — cuts through the dark like a laser
//   - High contrast foreground (#E8E8E8) — crisp, no eye strain
//   - Title bar nearly invisible — merges with the terminal, all focus on code
//   - Syntax colors chosen for maximum distinguishability at a glance
//   - Bold UI text throughout — every label punches
//   - Selection: deep electric teal, unmistakable

pub const DEVELOPER: Theme = Theme {
    id: "developer",
    name: "Developer",
    title_bar_bg: Color::rgb(8, 8, 8),
    title_bar_text: Color::rgb(180, 180, 180),
    window_border: Color::rgb(25, 25, 30),
    bg: Color::rgb(10, 10, 10),
    fg: Color::rgb(232, 232, 232),
    cursor: Color::rgb(0, 229, 255),
    selection_bg: Color::rgb(0, 60, 70),
    selection_fg: Color::rgb(255, 255, 255),
    ansi: [
        Color::rgb(18, 18, 18),       // black
        Color::rgb(255, 82, 82),      // red — Material A200
        Color::rgb(105, 240, 174),    // green — mint
        Color::rgb(255, 213, 79),     // yellow — warm gold
        Color::rgb(68, 138, 255),     // blue — Material A200
        Color::rgb(234, 128, 252),    // magenta — electric purple
        Color::rgb(0, 229, 255),      // cyan — electric, matches cursor
        Color::rgb(224, 224, 224),    // white
        Color::rgb(66, 66, 66),       // bright black — visible comments
        Color::rgb(255, 138, 128),    // bright red
        Color::rgb(129, 255, 196),    // bright green
        Color::rgb(255, 228, 130),    // bright yellow
        Color::rgb(130, 177, 255),    // bright blue
        Color::rgb(244, 168, 255),    // bright magenta
        Color::rgb(77, 240, 255),     // bright cyan
        Color::rgb(250, 250, 250),    // bright white
    ],
    bold_ui: true,
};
