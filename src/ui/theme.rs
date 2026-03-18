/// ClaudeTerm theme system
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
    selection_bg: Color::rgba(209, 142, 97, 80),
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
    selection_bg: Color::rgba(180, 100, 60, 60),
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
};

pub const MIDNIGHT: Theme = Theme {
    id: "midnight",
    name: "Midnight",
    title_bar_bg: Color::rgb(10, 10, 18),
    title_bar_text: Color::rgb(140, 160, 200),
    window_border: Color::rgb(25, 30, 50),
    bg: Color::rgb(5, 5, 12),
    fg: Color::rgb(180, 195, 230),
    cursor: Color::rgb(100, 140, 255),
    selection_bg: Color::rgba(100, 140, 255, 60),
    selection_fg: Color::rgb(230, 235, 255),
    ansi: [
        Color::rgb(15, 15, 25),
        Color::rgb(255, 85, 110),
        Color::rgb(80, 220, 140),
        Color::rgb(240, 200, 80),
        Color::rgb(80, 140, 255),
        Color::rgb(190, 120, 255),
        Color::rgb(80, 210, 230),
        Color::rgb(180, 190, 220),
        Color::rgb(60, 65, 90),
        Color::rgb(255, 120, 140),
        Color::rgb(120, 240, 170),
        Color::rgb(255, 220, 110),
        Color::rgb(120, 170, 255),
        Color::rgb(210, 150, 255),
        Color::rgb(120, 230, 245),
        Color::rgb(220, 225, 245),
    ],
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
    selection_bg: Color::rgba(7, 54, 66, 200),
    selection_fg: Color::rgb(147, 161, 161),
    ansi: [
        Color::rgb(7, 54, 66),
        Color::rgb(220, 50, 47),
        Color::rgb(133, 153, 0),
        Color::rgb(181, 137, 0),
        Color::rgb(38, 139, 210),
        Color::rgb(211, 54, 130),
        Color::rgb(42, 161, 152),
        Color::rgb(238, 232, 213),
        Color::rgb(0, 43, 54),
        Color::rgb(203, 75, 22),
        Color::rgb(88, 110, 117),
        Color::rgb(101, 123, 131),
        Color::rgb(131, 148, 150),
        Color::rgb(108, 113, 196),
        Color::rgb(147, 161, 161),
        Color::rgb(253, 246, 227),
    ],
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
    selection_bg: Color::rgba(68, 71, 90, 200),
    selection_fg: Color::rgb(248, 248, 242),
    ansi: [
        Color::rgb(33, 34, 44),
        Color::rgb(255, 85, 85),
        Color::rgb(80, 250, 123),
        Color::rgb(241, 250, 140),
        Color::rgb(98, 114, 164),
        Color::rgb(255, 121, 198),
        Color::rgb(139, 233, 253),
        Color::rgb(248, 248, 242),
        Color::rgb(98, 114, 164),
        Color::rgb(255, 110, 110),
        Color::rgb(105, 255, 148),
        Color::rgb(246, 255, 165),
        Color::rgb(123, 139, 189),
        Color::rgb(255, 146, 213),
        Color::rgb(164, 248, 255),
        Color::rgb(255, 255, 255),
    ],
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
    selection_bg: Color::rgba(67, 76, 94, 200),
    selection_fg: Color::rgb(236, 239, 244),
    ansi: [
        Color::rgb(59, 66, 82),
        Color::rgb(191, 97, 106),
        Color::rgb(163, 190, 140),
        Color::rgb(235, 203, 139),
        Color::rgb(129, 161, 193),
        Color::rgb(180, 142, 173),
        Color::rgb(136, 192, 208),
        Color::rgb(229, 233, 240),
        Color::rgb(76, 86, 106),
        Color::rgb(191, 97, 106),
        Color::rgb(163, 190, 140),
        Color::rgb(235, 203, 139),
        Color::rgb(129, 161, 193),
        Color::rgb(180, 142, 173),
        Color::rgb(143, 188, 187),
        Color::rgb(236, 239, 244),
    ],
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
    selection_bg: Color::rgba(87, 82, 74, 200),
    selection_fg: Color::rgb(252, 252, 250),
    ansi: [
        Color::rgb(55, 53, 50),
        Color::rgb(255, 97, 136),
        Color::rgb(169, 220, 118),
        Color::rgb(255, 216, 102),
        Color::rgb(120, 220, 232),
        Color::rgb(171, 157, 242),
        Color::rgb(120, 220, 232),
        Color::rgb(252, 252, 250),
        Color::rgb(114, 109, 103),
        Color::rgb(255, 97, 136),
        Color::rgb(169, 220, 118),
        Color::rgb(255, 216, 102),
        Color::rgb(120, 220, 232),
        Color::rgb(171, 157, 242),
        Color::rgb(120, 220, 232),
        Color::rgb(252, 252, 250),
    ],
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
    selection_bg: Color::rgba(69, 65, 57, 200),
    selection_fg: Color::rgb(235, 219, 178),
    ansi: [
        Color::rgb(40, 40, 40),
        Color::rgb(204, 36, 29),
        Color::rgb(152, 151, 26),
        Color::rgb(215, 153, 33),
        Color::rgb(69, 133, 136),
        Color::rgb(177, 98, 134),
        Color::rgb(104, 157, 106),
        Color::rgb(168, 153, 132),
        Color::rgb(146, 131, 116),
        Color::rgb(251, 73, 52),
        Color::rgb(184, 187, 38),
        Color::rgb(250, 189, 47),
        Color::rgb(131, 165, 152),
        Color::rgb(211, 134, 155),
        Color::rgb(142, 192, 124),
        Color::rgb(235, 219, 178),
    ],
};
