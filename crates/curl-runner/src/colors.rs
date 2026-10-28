//! ANSI colors (`core/format/formatters.ts` `ANSI_COLORS`/`colorize`,
//! `utils/colors.ts`).
//!
//! TS always colors. Rust does too, except when `NO_COLOR` is set to a
//! non-empty value (logged in CHANGES-FROM-TS.md); tests rely on that.

#[allow(dead_code)] // full TS table; later ports use more colors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Reset,
    Bright,
    Dim,
    Underscore,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BgBlack,
    BgRed,
    BgGreen,
    BgYellow,
    BgBlue,
    BgMagenta,
    BgCyan,
    BgWhite,
}

impl Color {
    pub const fn code(self) -> &'static str {
        match self {
            Color::Reset => "\x1b[0m",
            Color::Bright => "\x1b[1m",
            Color::Dim => "\x1b[2m",
            Color::Underscore => "\x1b[4m",
            Color::Black => "\x1b[30m",
            Color::Red => "\x1b[31m",
            Color::Green => "\x1b[32m",
            Color::Yellow => "\x1b[33m",
            Color::Blue => "\x1b[34m",
            Color::Magenta => "\x1b[35m",
            Color::Cyan => "\x1b[36m",
            Color::White => "\x1b[37m",
            Color::BgBlack => "\x1b[40m",
            Color::BgRed => "\x1b[41m",
            Color::BgGreen => "\x1b[42m",
            Color::BgYellow => "\x1b[43m",
            Color::BgBlue => "\x1b[44m",
            Color::BgMagenta => "\x1b[45m",
            Color::BgCyan => "\x1b[46m",
            Color::BgWhite => "\x1b[47m",
        }
    }
}

/// TS `colorize`: `{code}{text}{reset}`.
pub fn colorize(text: &str, color: Color) -> String {
    format!("{}{text}{}", color.code(), Color::Reset.code())
}

/// Whether output is colored: on unless `NO_COLOR` is non-empty.
pub fn enabled_from_env(env: impl Fn(&str) -> Option<String>) -> bool {
    env("NO_COLOR").is_none_or(|v| v.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    // formatters.test.ts: ANSI_COLORS / colorize
    #[test]
    fn codes_match_ts() {
        assert_eq!(Color::Reset.code(), "\x1b[0m");
        assert_eq!(Color::Red.code(), "\x1b[31m");
        assert_eq!(Color::Green.code(), "\x1b[32m");
        assert_eq!(Color::Blue.code(), "\x1b[34m");
        assert_eq!(Color::BgRed.code(), "\x1b[41m");
        assert_eq!(Color::BgGreen.code(), "\x1b[42m");
    }

    #[test]
    fn colorize_wraps_text() {
        assert_eq!(colorize("hello", Color::Red), "\x1b[31mhello\x1b[0m");
        assert!(colorize("test", Color::Green).contains("\x1b[32m"));
        assert!(colorize("test", Color::Blue).contains("\x1b[34m"));
        assert!(colorize("test", Color::Yellow).contains("\x1b[33m"));
        // dry-run-logger.test.ts: cyan DRY-RUN
        let c = colorize("DRY-RUN", Color::Cyan);
        assert!(c.contains("\x1b[36m") && c.contains("DRY-RUN") && c.contains("\x1b[0m"));
    }

    #[test]
    fn no_color_env() {
        assert!(enabled_from_env(|_| None));
        assert!(enabled_from_env(|_| Some(String::new())));
        assert!(!enabled_from_env(|_| Some("1".into())));
    }
}
