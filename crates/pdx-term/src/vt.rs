//! Hand-written VT100/xterm parser. No external crate is used.
//!
//! [`VtParser::process`] takes an arbitrary byte chunk and returns the
//! [`Action`]s decoded from it. Feed those actions to
//! [`crate::grid::TerminalGrid::apply`]. A chunk may split a UTF-8 code point
//! or an escape sequence at any byte boundary: incomplete sequences are kept
//! in the parser's buffers until later chunks complete them.
//!
//! Coordinate convention used by the emitted actions:
//! - `CursorTo(row, col)` is zero-based `(row, column)`.
//! - `CursorMove(dx, dy)` is relative; positive `dy` moves down.
//! - `SetScrollRegion(top, bottom)` is zero-based and inclusive.
//!
//! The parser keeps a lightweight shadow cursor so that column-absolute
//! moves (`CSI G`) can emit an absolute `CursorTo`. The shadow is a best
//! effort approximation: autowrap and scroll-region behaviour is finalized by
//! the grid, which is the source of truth for the real cursor position.

use serde::{Deserialize, Serialize};

/// A color as carried by an SGR sequence.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Color {
    /// A palette index (`0..=255`), including the bright 16-color entries.
    Indexed(u8),
    /// 24-bit RGB from `38;2;r;g;b` / `48;2;r;g;b`.
    Rgb(u8, u8, u8),
    /// Use the terminal default.
    #[default]
    Default,
}

/// The current SGR pen. Each `CSI ... m` replaces the pen wholesale; the
/// parser accumulates successive SGR updates internally before emitting.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct Style {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub reverse: bool,
}

/// A decoded terminal action.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Print(String),
    Newline,
    CarriageReturn,
    Backspace,
    Tab,
    EraseDisplay(u8),
    EraseLine(u8),
    /// Zero-based `(row, col)`.
    CursorTo(u16, u16),
    /// Relative `(dx, dy)`; positive `dy` moves down.
    CursorMove(i32, i32),
    SetSgr(Style),
    SetTitle(String),
    AltScreen(bool),
    BracketedPaste(bool),
    /// Zero-based inclusive `(top, bottom)`.
    SetScrollRegion(u16, u16),
    Bell,
    Reset,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Ground,
    Escape,
    Csi,
    Osc,
    /// DCS/SOS/PM/APC string: consumed and discarded until ST or CAN/SUB.
    IgnoreString,
}

/// Incremental VT parser.
pub struct VtParser {
    cols: u16,
    rows: u16,
    state: State,
    utf8: Vec<u8>,
    csi: Vec<u8>,
    osc: Vec<u8>,
    title: Option<String>,
    sgr: Style,
    /// Shadow cursor as `(col, row)`.
    cursor_col: u16,
    cursor_row: u16,
    /// An ESC seen inside an OSC/IgnoreString sequence (ST = ESC \).
    string_esc: bool,
}

impl VtParser {
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            cols: cols.max(1),
            rows: rows.max(1),
            state: State::Ground,
            utf8: Vec::new(),
            csi: Vec::new(),
            osc: Vec::new(),
            title: None,
            sgr: Style::default(),
            cursor_col: 0,
            cursor_row: 0,
            string_esc: false,
        }
    }

    /// Decode a byte chunk into actions. Chunks may be split arbitrarily.
    pub fn process(&mut self, bytes: &[u8]) -> Vec<Action> {
        let mut out = Vec::new();
        for &byte in bytes {
            match self.state {
                State::Ground => self.ground(byte, &mut out),
                State::Escape => self.escape(byte, &mut out),
                State::Csi => self.csi(byte, &mut out),
                State::Osc => self.osc(byte, &mut out),
                State::IgnoreString => self.ignore_string(byte, &mut out),
            }
        }
        out
    }

    /// Last window title reported by OSC 0 or OSC 2.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Dimensions supplied at construction (clamped to at least 1).
    pub fn dimensions(&self) -> (u16, u16) {
        (self.cols, self.rows)
    }

    /// Current SGR pen accumulated so far.
    pub fn style(&self) -> Style {
        self.sgr
    }

    /// Shadow cursor as `(col, row)`.
    pub fn shadow_cursor(&self) -> (u16, u16) {
        (self.cursor_col, self.cursor_row)
    }

    fn ground(&mut self, byte: u8, out: &mut Vec<Action>) {
        match byte {
            0x00 => {}
            0x07 => {
                self.flush(out);
                out.push(Action::Bell);
            }
            0x08 => {
                self.flush(out);
                out.push(Action::Backspace);
                self.cursor_col = self.cursor_col.saturating_sub(1);
            }
            0x09 => {
                self.flush(out);
                out.push(Action::Tab);
                self.advance_tab();
            }
            0x0A..=0x0C => {
                self.flush(out);
                out.push(Action::Newline);
                self.cursor_row = self
                    .cursor_row
                    .saturating_add(1)
                    .min(self.rows.saturating_sub(1));
            }
            0x0D => {
                self.flush(out);
                out.push(Action::CarriageReturn);
                self.cursor_col = 0;
            }
            0x18 | 0x1A => {
                // CAN / SUB abort any pending sequence.
                self.utf8.clear();
                self.state = State::Ground;
            }
            0x1B => {
                self.flush(out);
                self.state = State::Escape;
            }
            0x7F => {}
            _ => {
                if self.utf8.len() < 4096 {
                    self.utf8.push(byte);
                }
            }
        }
    }

    /// Flush accumulated printable bytes as one lossy-decoded string.
    fn flush(&mut self, out: &mut Vec<Action>) {
        if self.utf8.is_empty() {
            return;
        }
        let text = String::from_utf8_lossy(&self.utf8).into_owned();
        self.utf8.clear();
        let count = text.chars().count().min(u16::MAX as usize) as u16;
        self.cursor_col = self
            .cursor_col
            .saturating_add(count)
            .min(self.cols.saturating_sub(1));
        out.push(Action::Print(text));
    }

    fn advance_tab(&mut self) {
        let next = ((self.cursor_col as u32 / 8) + 1) * 8;
        self.cursor_col = next.min(self.cols.saturating_sub(1) as u32) as u16;
    }

    fn escape(&mut self, byte: u8, out: &mut Vec<Action>) {
        match byte {
            b'[' => {
                self.csi.clear();
                self.state = State::Csi;
            }
            b']' => {
                self.osc.clear();
                self.string_esc = false;
                self.state = State::Osc;
            }
            b'P' | b'X' | b'^' | b'_' => {
                self.string_esc = false;
                self.state = State::IgnoreString;
            }
            b'c' => {
                // RIS: full reset.
                out.push(Action::Reset);
                self.sgr = Style::default();
                self.title = None;
                self.cursor_col = 0;
                self.cursor_row = 0;
                self.state = State::Ground;
            }
            0x1B => self.state = State::Escape,
            0x18 | 0x1A => self.state = State::Ground,
            _ => self.state = State::Ground,
        }
    }

    fn csi(&mut self, byte: u8, out: &mut Vec<Action>) {
        // Final bytes occupy 0x40..=0x7E.
        if (0x40..=0x7E).contains(&byte) {
            self.csi.push(byte);
            if let Some(&final_byte) = self.csi.last() {
                self.dispatch_csi(final_byte, out);
            }
            self.csi.clear();
            self.state = State::Ground;
            return;
        }
        match byte {
            0x1B => {
                self.csi.clear();
                self.state = State::Escape;
            }
            0x18 | 0x1A => {
                self.csi.clear();
                self.state = State::Ground;
            }
            // Parameter bytes 0x30..=0x3F and intermediates 0x20..=0x2F.
            0x20..=0x3F if self.csi.len() < 128 => self.csi.push(byte),
            _ => {}
        }
    }

    fn dispatch_csi(&mut self, final_byte: u8, out: &mut Vec<Action>) {
        let body_len = self.csi.len().saturating_sub(1);
        let body = &self.csi[..body_len];
        let private = body.first() == Some(&b'?');
        let nums = parse_params(body);
        let param =
            |index: usize, default: u16| -> u16 { nums.get(index).copied().unwrap_or(default) };

        match final_byte {
            b'A' => {
                let n = param(0, 1);
                out.push(Action::CursorMove(0, -(n as i32)));
                self.cursor_row = self.cursor_row.saturating_sub(n);
            }
            b'B' => {
                let n = param(0, 1);
                out.push(Action::CursorMove(0, n as i32));
                self.cursor_row = self.cursor_row.saturating_add(n).min(self.rows - 1);
            }
            b'C' => {
                let n = param(0, 1);
                out.push(Action::CursorMove(n as i32, 0));
                self.cursor_col = self.cursor_col.saturating_add(n).min(self.cols - 1);
            }
            b'D' => {
                let n = param(0, 1);
                out.push(Action::CursorMove(-(n as i32), 0));
                self.cursor_col = self.cursor_col.saturating_sub(n);
            }
            b'G' | b'`' => {
                let col = param(0, 1).saturating_sub(1).min(self.cols - 1);
                self.cursor_col = col;
                out.push(Action::CursorTo(self.cursor_row, col));
            }
            b'H' | b'f' => {
                let row = param(0, 1).saturating_sub(1).min(self.rows - 1);
                let col = param(1, 1).saturating_sub(1).min(self.cols - 1);
                self.cursor_row = row;
                self.cursor_col = col;
                out.push(Action::CursorTo(row, col));
            }
            b'J' => out.push(Action::EraseDisplay(param(0, 0).min(2) as u8)),
            b'K' => out.push(Action::EraseLine(param(0, 0).min(2) as u8)),
            b'm' => {
                self.sgr = apply_sgr(self.sgr, &nums);
                out.push(Action::SetSgr(self.sgr));
            }
            b'r' => {
                let top = param(0, 1).saturating_sub(1).min(self.rows - 1);
                let bottom = param(1, self.rows).saturating_sub(1).min(self.rows - 1);
                if top <= bottom {
                    out.push(Action::SetScrollRegion(top, bottom));
                }
            }
            b'h' if private => {
                for number in &nums {
                    match *number {
                        1049 => out.push(Action::AltScreen(true)),
                        2004 => out.push(Action::BracketedPaste(true)),
                        _ => {}
                    }
                }
            }
            b'l' if private => {
                for number in &nums {
                    match *number {
                        1049 => out.push(Action::AltScreen(false)),
                        2004 => out.push(Action::BracketedPaste(false)),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    fn osc(&mut self, byte: u8, out: &mut Vec<Action>) {
        if self.string_esc {
            self.string_esc = false;
            if byte == b'\\' {
                self.finish_osc(out);
            } else {
                // ESC followed by something other than `\`: abort the OSC
                // and reinterpret the byte as a new escape introducer.
                self.state = State::Escape;
                self.escape(byte, out);
            }
            return;
        }
        match byte {
            0x07 => self.finish_osc(out),
            0x1B => self.string_esc = true,
            0x18 | 0x1A => {
                self.osc.clear();
                self.state = State::Ground;
            }
            _ => {
                if self.osc.len() < 4096 {
                    self.osc.push(byte);
                }
            }
        }
    }

    fn finish_osc(&mut self, out: &mut Vec<Action>) {
        let raw = String::from_utf8_lossy(&self.osc).into_owned();
        self.osc.clear();
        self.state = State::Ground;
        if let Some((code, value)) = raw.split_once(';') {
            if code == "0" || code == "2" {
                self.title = Some(value.to_string());
                out.push(Action::SetTitle(value.to_string()));
            }
        }
    }

    fn ignore_string(&mut self, byte: u8, out: &mut Vec<Action>) {
        if self.string_esc {
            self.string_esc = false;
            if byte != b'\\' {
                self.state = State::Escape;
                self.escape(byte, out);
            } else {
                self.state = State::Ground;
            }
            return;
        }
        match byte {
            0x07 | 0x18 | 0x1A => self.state = State::Ground,
            0x1B => self.string_esc = true,
            _ => {}
        }
    }
}

/// Parse `;`-separated decimal parameters. Empty parameters become 0;
/// non-digit bytes (`?`, `:`) are ignored. Values saturate at `u16::MAX`.
fn parse_params(body: &[u8]) -> Vec<u16> {
    let mut out = Vec::new();
    let mut current: u32 = 0;
    let mut have_digit = false;
    for &byte in body {
        match byte {
            b'0'..=b'9' => {
                current = current
                    .saturating_mul(10)
                    .saturating_add((byte - b'0') as u32);
                have_digit = true;
            }
            b';' => {
                out.push(current.min(u16::MAX as u32) as u16);
                current = 0;
                have_digit = false;
            }
            _ => {}
        }
    }
    if have_digit || body.last() == Some(&b';') {
        out.push(current.min(u16::MAX as u32) as u16);
    }
    out
}

/// Apply a list of SGR parameters to a pen in order.
fn apply_sgr(mut style: Style, nums: &[u16]) -> Style {
    if nums.is_empty() {
        return Style::default();
    }
    let mut index = 0;
    while index < nums.len() {
        match nums[index] {
            0 => style = Style::default(),
            1 => style.bold = true,
            2 => style.dim = true,
            3 => style.italic = true,
            4 => style.underline = true,
            7 => style.reverse = true,
            22 => {
                style.bold = false;
                style.dim = false;
            }
            23 => style.italic = false,
            24 => style.underline = false,
            27 => style.reverse = false,
            39 => style.fg = None,
            49 => style.bg = None,
            30..=37 => style.fg = Some(Color::Indexed((nums[index] - 30) as u8)),
            90..=97 => style.fg = Some(Color::Indexed((nums[index] - 90 + 8) as u8)),
            40..=47 => style.bg = Some(Color::Indexed((nums[index] - 40) as u8)),
            100..=107 => style.bg = Some(Color::Indexed((nums[index] - 100 + 8) as u8)),
            38 => apply_extended_color(nums, &mut index, &mut style, false),
            48 => apply_extended_color(nums, &mut index, &mut style, true),
            _ => {}
        }
        index += 1;
    }
    style
}

/// Consume an extended-color SGR (`38`/`48`) starting at `index`.
fn apply_extended_color(nums: &[u16], index: &mut usize, style: &mut Style, background: bool) {
    if *index + 1 >= nums.len() {
        return;
    }
    match nums[*index + 1] {
        5 if *index + 2 < nums.len() => {
            let color = Some(Color::Indexed(nums[*index + 2] as u8));
            if background {
                style.bg = color;
            } else {
                style.fg = color;
            }
            *index += 2;
        }
        2 if *index + 4 < nums.len() => {
            let color = Some(Color::Rgb(
                nums[*index + 2] as u8,
                nums[*index + 3] as u8,
                nums[*index + 4] as u8,
            ));
            if background {
                style.bg = color;
            } else {
                style.fg = color;
            }
            *index += 4;
        }
        _ => {}
    }
}
