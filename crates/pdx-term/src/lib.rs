//! pdx-term — VT100/xterm parsing, a screen grid and a Unix PTY.
//!
//! Everything here builds and unit-tests on host x86_64. The PTY module is
//! real on Unix (Android/Linux) and returns a typed capability error on other
//! hosts.

pub mod grid;
pub mod pty;
pub mod vt;

pub use grid::{Cell, GridSnapshot, Rect, TerminalGrid, SCROLLBACK_LIMIT};
pub use pty::Pty;
pub use vt::{Action, Color, Style, VtParser};

#[cfg(test)]
mod tests {
    use super::*;
    use pdx_core::ErrorKind;

    fn sgr_from(actions: &[Action]) -> Style {
        actions
            .iter()
            .find_map(|action| match action {
                Action::SetSgr(style) => Some(*style),
                _ => None,
            })
            .expect("expected a SetSgr action")
    }

    #[test]
    fn csi_sgr_16_color_and_attributes() {
        let mut parser = VtParser::new(80, 24);
        let actions = parser.process(b"\x1b[1;31m");
        let style = sgr_from(&actions);
        assert!(style.bold);
        assert_eq!(style.fg, Some(Color::Indexed(1)));
        assert_eq!(style.bg, None);

        let actions = parser.process(b"\x1b[0m");
        assert_eq!(sgr_from(&actions), Style::default());
    }

    #[test]
    fn csi_sgr_256_color_and_truecolor() {
        let mut parser = VtParser::new(80, 24);
        let actions = parser.process(b"\x1b[38;5;196;48;2;10;20;30;3;4;7m");
        let style = sgr_from(&actions);
        assert_eq!(style.fg, Some(Color::Indexed(196)));
        assert_eq!(style.bg, Some(Color::Rgb(10, 20, 30)));
        assert!(style.italic);
        assert!(style.underline);
        assert!(style.reverse);
    }

    #[test]
    fn osc_title_bel_and_st() {
        let mut parser = VtParser::new(80, 24);
        parser.process(b"\x1b]0;hello\x07");
        assert_eq!(parser.title(), Some("hello"));
        parser.process(b"\x1b]2;world\x1b\\");
        assert_eq!(parser.title(), Some("world"));
    }

    #[test]
    fn alt_screen_toggle_saves_and_restores() {
        let mut grid = TerminalGrid::new(4, 3);
        grid.apply(Action::Print("abc".to_string()));
        grid.apply(Action::AltScreen(true));
        assert!(grid.alt_screen());
        assert!(grid.snapshot().lines[0].iter().all(|c| c.ch == ' '));
        grid.apply(Action::Print("X".to_string()));
        grid.apply(Action::AltScreen(false));
        assert!(!grid.alt_screen());
        let snapshot = grid.snapshot();
        assert_eq!(snapshot.lines[0][0].ch, 'a');
        assert_eq!(snapshot.lines[0][1].ch, 'b');
    }

    #[test]
    fn erase_display_and_line() {
        let mut grid = TerminalGrid::new(4, 2);
        grid.apply(Action::CursorTo(0, 0));
        grid.apply(Action::Print("abcd".to_string()));
        grid.apply(Action::EraseDisplay(2));
        assert!(grid
            .snapshot()
            .lines
            .iter()
            .all(|row| row.iter().all(|cell| cell.ch == ' ')));

        grid.apply(Action::CursorTo(0, 0));
        grid.apply(Action::Print("WXYZ".to_string()));
        grid.apply(Action::CursorTo(1, 0));
        grid.apply(Action::EraseLine(0));
        let snapshot = grid.snapshot();
        assert_eq!(snapshot.lines[0][0].ch, 'W');
        assert!(snapshot.lines[1].iter().all(|cell| cell.ch == ' '));
    }

    #[test]
    fn cursor_to_and_relative_move() {
        let mut grid = TerminalGrid::new(10, 5);
        grid.apply(Action::CursorTo(2, 3));
        assert_eq!(grid.cursor(), (3, 2));
        grid.apply(Action::CursorMove(-2, 1));
        assert_eq!(grid.cursor(), (1, 3));
        grid.apply(Action::CursorMove(-99, -99));
        assert_eq!(grid.cursor(), (0, 0));
    }

    #[test]
    fn tab_advances_to_eight_column_stops() {
        let mut grid = TerminalGrid::new(40, 2);
        grid.apply(Action::Tab);
        assert_eq!(grid.cursor().0, 8);
        grid.apply(Action::Tab);
        assert_eq!(grid.cursor().0, 16);
    }

    #[test]
    fn scrollback_is_bounded() {
        let mut grid = TerminalGrid::new(4, 2);
        for _ in 0..(SCROLLBACK_LIMIT + 5) {
            grid.apply(Action::Newline);
        }
        assert_eq!(grid.scrollback_len(), SCROLLBACK_LIMIT);
    }

    #[test]
    fn resize_preserves_scrollback() {
        let mut grid = TerminalGrid::new(8, 3);
        for _ in 0..5 {
            grid.apply(Action::Newline);
        }
        let before = grid.scrollback_len();
        assert!(before > 0);
        grid.resize(6, 2);
        assert!(grid.scrollback_len() >= before);
        let snapshot = grid.snapshot();
        assert_eq!(snapshot.cols, 6);
        assert_eq!(snapshot.rows, 2);
        assert_eq!(snapshot.lines.len(), 2);
    }

    #[test]
    fn dirty_regions_coalesce_and_clear() {
        let mut grid = TerminalGrid::new(4, 2);
        grid.apply(Action::Print("ab".to_string()));
        let regions = grid.dirty_regions();
        assert_eq!(regions.len(), 1);
        assert_eq!(
            regions[0],
            Rect {
                x: 0,
                y: 0,
                width: 4,
                height: 1
            }
        );
        assert!(grid.dirty_regions().is_empty());
    }

    #[cfg(not(unix))]
    #[test]
    fn pty_unavailable_on_non_unix_is_typed() {
        let error = Pty::spawn("/bin/sh", &[], &[], 24, 80).unwrap_err();
        assert_eq!(error.kind, ErrorKind::CapabilityFailure);
        assert!(error.message.contains("Unix"));
    }
}
