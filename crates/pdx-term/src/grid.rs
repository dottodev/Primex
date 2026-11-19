//! A screen grid that consumes [`Action`]s from the VT parser.
//!
//! Wide and combining characters are approximated: each cell holds a single
//! `char`, and zero-width combining marks are skipped rather than composed
//! onto the preceding cell. East Asian wide glyphs therefore occupy one cell
//! instead of two. This keeps the model simple and deterministic; a future
//! grapheme-aware renderer can replace `is_zero_width` without changing the
//! grid's API.

use crate::vt::{Action, Style};
use serde::{Deserialize, Serialize};
use std::cell::Cell as FlagCell;
use std::collections::VecDeque;

/// Maximum number of scrolled-off lines retained.
pub const SCROLLBACK_LIMIT: usize = 10_000;

/// One screen cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell {
    pub ch: char,
    pub style: Style,
    pub dirty: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            style: Style::default(),
            dirty: false,
        }
    }
}

/// A changed full-width row range: `x..x+width` by `y..y+height`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

/// An immutable view of the visible screen.
#[derive(Clone, Debug)]
pub struct GridSnapshot {
    pub cols: u16,
    pub rows: u16,
    pub lines: Vec<Vec<Cell>>,
    /// Cursor as `(col, row)`.
    pub cursor: (u16, u16),
    pub title: Option<String>,
}

/// A mutable terminal screen with bounded scrollback and an alternate screen.
pub struct TerminalGrid {
    cols: u16,
    rows: u16,
    cells: Vec<Cell>,
    /// Cursor as `(col, row)`.
    cursor: (u16, u16),
    scroll_top: u16,
    scroll_bottom: u16,
    scrollback: VecDeque<Vec<Cell>>,
    alt_screen: bool,
    saved: Option<(Vec<Cell>, (u16, u16))>,
    current_style: Style,
    title: Option<String>,
    bracketed_paste: bool,
    /// Row-level dirty markers consumed by `dirty_regions`. Interior
    /// mutability lets `dirty_regions` keep its `&self` signature while still
    /// clearing the markers it returns; `Cell::dirty` remains a persistent
    /// per-cell hint for other consumers.
    dirty_rows: FlagCell<Vec<bool>>,
}

impl TerminalGrid {
    pub fn new(cols: u16, rows: u16) -> Self {
        let cols = cols.max(1);
        let rows = rows.max(1);
        Self {
            cells: vec![Cell::default(); cols as usize * rows as usize],
            cols,
            rows,
            cursor: (0, 0),
            scroll_top: 0,
            scroll_bottom: rows - 1,
            scrollback: VecDeque::new(),
            alt_screen: false,
            saved: None,
            current_style: Style::default(),
            title: None,
            bracketed_paste: false,
            dirty_rows: FlagCell::new(vec![false; rows as usize]),
        }
    }

    /// Apply one decoded parser action.
    pub fn apply(&mut self, action: Action) {
        match action {
            Action::Print(text) => self.print(&text),
            Action::Newline => self.linefeed(),
            Action::CarriageReturn => self.cursor.0 = 0,
            Action::Backspace => self.cursor.0 = self.cursor.0.saturating_sub(1),
            Action::Tab => {
                let next = ((self.cursor.0 as u32 / 8) + 1) * 8;
                self.cursor.0 = next.min(self.cols.saturating_sub(1) as u32) as u16;
            }
            Action::EraseDisplay(mode) => self.erase_display(mode),
            Action::EraseLine(mode) => self.erase_line(mode),
            Action::CursorTo(row, col) => {
                self.cursor.0 = col.min(self.cols - 1);
                self.cursor.1 = row.min(self.rows - 1);
            }
            Action::CursorMove(dx, dy) => self.move_cursor(dx, dy),
            Action::SetSgr(style) => self.current_style = style,
            Action::SetTitle(title) => self.title = Some(title),
            Action::AltScreen(active) => self.set_alt_screen(active),
            Action::BracketedPaste(active) => self.bracketed_paste = active,
            Action::SetScrollRegion(top, bottom) => {
                let top = top.min(self.rows - 1);
                let bottom = bottom.min(self.rows - 1);
                if top <= bottom {
                    self.scroll_top = top;
                    self.scroll_bottom = bottom;
                }
            }
            Action::Bell => {}
            Action::Reset => self.reset_state(),
        }
    }

    /// Resize the screen, reflowing history to the new width.
    ///
    /// Scrollback lines wider than the new column count are wrapped into
    /// several lines; lines narrower than it are padded and kept intact
    /// (never joined). The oldest lines are dropped once the 10,000-line cap
    /// is exceeded. Nothing already in scrollback is discarded except by
    /// that cap.
    pub fn resize(&mut self, cols: u16, rows: u16) {
        let cols = cols.max(1);
        let rows = rows.max(1);
        if cols == self.cols && rows == self.rows {
            return;
        }
        let old_cols = self.cols as usize;
        let old_rows = self.rows as usize;
        let new_cols = cols as usize;
        let new_rows = rows as usize;

        // Collect the entire history plus the visible screen as lines.
        let mut lines: Vec<Vec<Cell>> = Vec::with_capacity(self.scrollback.len() + old_rows);
        while let Some(line) = self.scrollback.pop_front() {
            lines.push(line);
        }
        for row in 0..old_rows {
            let start = row * old_cols;
            lines.push(self.cells[start..start + old_cols].to_vec());
        }

        let reflowed = reflow_lines(&lines, new_cols);
        // Everything that no longer fits on screen becomes scrollback.
        let visible_count = reflowed.len().min(new_rows);
        let history_count = reflowed.len() - visible_count;
        for line in &reflowed[..history_count] {
            self.push_scrollback(line.clone());
        }
        let new_cells = flatten_visible(&reflowed[history_count..], new_cols, new_rows);

        // Reflow the saved alternate screen the same way.
        let saved = self.saved.take().map(|(cells, cursor)| {
            let mut saved_lines = Vec::with_capacity(old_rows);
            for row in 0..old_rows {
                let start = row * old_cols;
                saved_lines.push(cells[start..start + old_cols].to_vec());
            }
            let reflowed_saved = reflow_lines(&saved_lines, new_cols);
            let saved_cells = flatten_visible(&reflowed_saved, new_cols, new_rows);
            let saved_cursor = (cursor.0.min(cols - 1), cursor.1.min(rows - 1));
            (saved_cells, saved_cursor)
        });

        self.cols = cols;
        self.rows = rows;
        self.cells = new_cells;
        self.cursor.0 = self.cursor.0.min(cols - 1);
        self.cursor.1 = self.cursor.1.min(rows - 1);
        self.scroll_top = 0;
        self.scroll_bottom = rows - 1;
        self.saved = saved;
        self.dirty_rows.set(vec![false; new_rows]);
        self.mark_all_dirty();
    }

    /// Immutable view of the visible screen.
    pub fn snapshot(&self) -> GridSnapshot {
        let mut lines = Vec::with_capacity(self.rows as usize);
        for row in 0..self.rows as usize {
            let start = row * self.cols as usize;
            lines.push(self.cells[start..start + self.cols as usize].to_vec());
        }
        GridSnapshot {
            cols: self.cols,
            rows: self.rows,
            lines,
            cursor: self.cursor,
            title: self.title.clone(),
        }
    }

    /// Return coalesced dirty row ranges and clear the row markers.
    pub fn dirty_regions(&self) -> Vec<Rect> {
        let mut flags = self.dirty_rows.take();
        if flags.len() < self.rows as usize {
            flags.resize(self.rows as usize, false);
        }
        let mut regions = Vec::new();
        let mut start: Option<u16> = None;
        for row in 0..self.rows {
            let is_dirty = flags.get(row as usize).copied().unwrap_or(false);
            if is_dirty {
                if start.is_none() {
                    start = Some(row);
                }
            } else if let Some(first) = start.take() {
                regions.push(Rect {
                    x: 0,
                    y: first,
                    width: self.cols,
                    height: row - first,
                });
            }
        }
        if let Some(first) = start {
            regions.push(Rect {
                x: 0,
                y: first,
                width: self.cols,
                height: self.rows - first,
            });
        }
        flags.fill(false);
        self.dirty_rows.set(flags);
        regions
    }

    pub fn scrollback_len(&self) -> usize {
        self.scrollback.len()
    }

    /// Enter or leave the alternate screen. Entering saves the primary
    /// screen and clears the alternate buffer; leaving restores it.
    pub fn set_alt_screen(&mut self, active: bool) {
        if active == self.alt_screen {
            return;
        }
        if active {
            self.saved = Some((self.cells.clone(), self.cursor));
            for cell in &mut self.cells {
                *cell = Cell::default();
            }
            self.cursor = (0, 0);
        } else if let Some((cells, cursor)) = self.saved.take() {
            self.cells = cells;
            self.cursor = cursor;
        }
        self.alt_screen = active;
        self.mark_all_dirty();
    }

    pub fn cols(&self) -> u16 {
        self.cols
    }

    pub fn rows(&self) -> u16 {
        self.rows
    }

    /// Cursor as `(col, row)`.
    pub fn cursor(&self) -> (u16, u16) {
        self.cursor
    }

    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    pub fn bracketed_paste(&self) -> bool {
        self.bracketed_paste
    }

    pub fn alt_screen(&self) -> bool {
        self.alt_screen
    }

    /// Scroll region as `(top, bottom)`, zero-based inclusive.
    pub fn scroll_region(&self) -> (u16, u16) {
        (self.scroll_top, self.scroll_bottom)
    }

    fn print(&mut self, text: &str) {
        for ch in text.chars() {
            if is_zero_width(ch) {
                continue;
            }
            if self.cursor.0 >= self.cols {
                self.cursor.0 = 0;
                self.linefeed();
            }
            let index = self.index(self.cursor.0, self.cursor.1);
            self.cells[index] = Cell {
                ch,
                style: self.current_style,
                dirty: true,
            };
            self.cursor.0 += 1;
        }
        let row = self.cursor.1;
        self.mark_row_dirty(row);
    }

    fn linefeed(&mut self) {
        if self.cursor.1 == self.scroll_bottom {
            self.scroll_up_one();
        } else if self.cursor.1 < self.rows - 1 {
            self.cursor.1 += 1;
            self.mark_row_dirty(self.cursor.1);
        }
    }

    fn scroll_up_one(&mut self) {
        let top = self.scroll_top as usize;
        let bottom = self.scroll_bottom as usize;
        let width = self.cols as usize;
        let line = self.cells[top * width..(top + 1) * width].to_vec();
        self.push_scrollback(line);
        for row in top..bottom {
            let dst = row * width;
            let src = (row + 1) * width;
            self.cells.copy_within(src..src + width, dst);
        }
        for index in bottom * width..(bottom + 1) * width {
            self.cells[index] = Cell::default();
        }
        for row in top..=bottom {
            self.mark_row_dirty(row as u16);
        }
    }

    fn push_scrollback(&mut self, line: Vec<Cell>) {
        self.scrollback.push_back(line);
        while self.scrollback.len() > SCROLLBACK_LIMIT {
            self.scrollback.pop_front();
        }
    }

    fn erase_display(&mut self, mode: u8) {
        let (col, row) = self.cursor;
        match mode {
            0 => {
                for r in row..self.rows {
                    self.erase_range(r, col, self.cols);
                }
            }
            1 => {
                for r in 0..=row {
                    let end = if r == row { col + 1 } else { self.cols };
                    self.erase_range(r, 0, end.min(self.cols));
                }
            }
            2 => {
                for r in 0..self.rows {
                    self.erase_range(r, 0, self.cols);
                }
            }
            _ => {}
        }
    }

    fn erase_line(&mut self, mode: u8) {
        let (col, row) = self.cursor;
        match mode {
            0 => self.erase_range(row, col, self.cols),
            1 => self.erase_range(row, 0, (col + 1).min(self.cols)),
            2 => self.erase_range(row, 0, self.cols),
            _ => {}
        }
    }

    fn erase_range(&mut self, row: u16, from: u16, to: u16) {
        if from >= to {
            return;
        }
        let width = self.cols as usize;
        let start = row as usize * width;
        for index in (start + from as usize)..(start + to as usize) {
            self.cells[index] = Cell {
                ch: ' ',
                style: self.current_style,
                dirty: true,
            };
        }
        self.mark_row_dirty(row);
    }

    fn move_cursor(&mut self, dx: i32, dy: i32) {
        let col = self.cursor.0 as i32 + dx;
        let row = self.cursor.1 as i32 + dy;
        self.cursor.0 = col.clamp(0, self.cols as i32 - 1) as u16;
        self.cursor.1 = row.clamp(0, self.rows as i32 - 1) as u16;
    }

    fn reset_state(&mut self) {
        self.current_style = Style::default();
        self.title = None;
        self.bracketed_paste = false;
        self.scroll_top = 0;
        self.scroll_bottom = self.rows - 1;
        self.cursor = (0, 0);
        self.alt_screen = false;
        self.saved = None;
        for cell in &mut self.cells {
            *cell = Cell::default();
        }
        self.mark_all_dirty();
    }

    fn index(&self, col: u16, row: u16) -> usize {
        row as usize * self.cols as usize + col as usize
    }

    fn mark_row_dirty(&self, row: u16) {
        let mut flags = self.dirty_rows.take();
        if (row as usize) < flags.len() {
            flags[row as usize] = true;
        }
        self.dirty_rows.set(flags);
    }

    fn mark_all_dirty(&self) {
        self.dirty_rows.set(vec![true; self.rows as usize]);
    }
}

/// Rewrap lines to a new width. Narrow lines are padded, wide lines split.
fn reflow_lines(lines: &[Vec<Cell>], width: usize) -> Vec<Vec<Cell>> {
    let mut out = Vec::with_capacity(lines.len());
    for line in lines {
        if line.len() <= width {
            let mut padded = line.clone();
            padded.resize(width, Cell::default());
            out.push(padded);
        } else {
            for chunk in line.chunks(width) {
                let mut padded = chunk.to_vec();
                padded.resize(width, Cell::default());
                out.push(padded);
            }
        }
    }
    out
}

/// Keep the last `rows` lines, flattened into a full cell buffer.
fn flatten_visible(lines: &[Vec<Cell>], cols: usize, rows: usize) -> Vec<Cell> {
    let mut out = vec![Cell::default(); cols * rows];
    let start = lines.len().saturating_sub(rows);
    for (index, line) in lines.iter().enumerate().skip(start).take(rows) {
        let dst = (index - start) * cols;
        out[dst..dst + cols].copy_from_slice(&line[..cols]);
    }
    out
}

/// Approximate zero-width characters (combining marks, ZWJ, variation
/// selectors). Full Unicode grapheme clustering is out of scope here.
fn is_zero_width(ch: char) -> bool {
    let code = ch as u32;
    (0x0300..=0x036F).contains(&code)
        || (0x0483..=0x0489).contains(&code)
        || (0x0591..=0x05BD).contains(&code)
        || (0x1AB0..=0x1AFF).contains(&code)
        || (0x1DC0..=0x1DFF).contains(&code)
        || (0x20D0..=0x20FF).contains(&code)
        || (0xFE00..=0xFE0F).contains(&code)
        || (0xFE20..=0xFE2F).contains(&code)
        || code == 0x200D
}
