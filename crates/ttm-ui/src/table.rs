//! `Table`: columns that fit the width, cut with `…` (never wrapped), two spaces between columns, no vertical rules.
//!
//! Width: fixed columns (no flex weight) take their content width within `min..=max`; flex columns share what is
//! left by weight, shrinking down to their `min` before cells are cut.

use crate::Render;
use crate::mode::Mode;
use crate::theme::{Token, paint};
use crate::width::{Align, display_width, pad, truncate};

const GAP: usize = 2;

#[derive(Debug, Clone)]
pub struct Col {
    header: String,
    min: Option<usize>,
    max: Option<usize>,
    flex: u16,
    align: Align,
}

impl Col {
    pub fn new(header: impl Into<String>) -> Self {
        Self {
            header: header.into(),
            min: None,
            max: None,
            flex: 0,
            align: Align::Left,
        }
    }

    pub fn min(mut self, n: usize) -> Self {
        self.min = Some(n);
        self
    }

    pub fn max(mut self, n: usize) -> Self {
        self.max = Some(n);
        self
    }

    /// Share of the free width (0 = fixed column).
    pub fn flex(mut self, weight: u16) -> Self {
        self.flex = weight;
        self
    }

    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    pub fn right(self) -> Self {
        self.align(Align::Right)
    }
}

/// One cell: text (may already hold colour codes, e.g. a rendered [`crate::Status`]) and an optional colour.
#[derive(Debug, Clone, Default)]
pub struct Cell {
    text: String,
    token: Option<Token>,
}

impl Cell {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            token: None,
        }
    }

    pub fn token(text: impl Into<String>, token: Token) -> Self {
        Self {
            text: text.into(),
            token: Some(token),
        }
    }

    pub fn empty() -> Self {
        Self::default()
    }
}

impl From<&str> for Cell {
    fn from(text: &str) -> Self {
        Cell::new(text)
    }
}

impl From<String> for Cell {
    fn from(text: String) -> Self {
        Cell::new(text)
    }
}

impl From<&String> for Cell {
    fn from(text: &String) -> Self {
        Cell::new(text.clone())
    }
}

#[derive(Debug, Clone)]
pub struct Table {
    cols: Vec<Col>,
    rows: Vec<Vec<Cell>>,
    empty: String,
}

impl Table {
    pub fn new(cols: &[Col]) -> Self {
        Self {
            cols: cols.to_vec(),
            rows: Vec::new(),
            empty: "Nothing to show.".into(),
        }
    }

    /// Adds a row; missing cells are empty, extra cells are dropped.
    pub fn row(mut self, cells: Vec<Cell>) -> Self {
        self.push(cells);
        self
    }

    pub fn push(&mut self, mut cells: Vec<Cell>) {
        cells.resize_with(self.cols.len(), Cell::empty);
        self.rows.push(cells);
    }

    /// What is printed instead of the table when it has no rows.
    pub fn empty_text(mut self, text: impl Into<String>) -> Self {
        self.empty = text.into();
        self
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Column widths for `total` columns of room.
    pub fn widths(&self, total: usize) -> Vec<usize> {
        let natural: Vec<usize> = self
            .cols
            .iter()
            .enumerate()
            .map(|(i, col)| {
                let content = self
                    .rows
                    .iter()
                    .map(|r| display_width(&r[i].text))
                    .chain(std::iter::once(display_width(&col.header)))
                    .max()
                    .unwrap_or(0);
                let lo = col.min.unwrap_or(0);
                let hi = col.max.unwrap_or(usize::MAX).max(lo);
                content.clamp(lo, hi)
            })
            .collect();
        let mut widths = natural.clone();
        let gaps = GAP * self.cols.len().saturating_sub(1);
        let fixed: usize = self
            .cols
            .iter()
            .zip(&natural)
            .filter(|(c, _)| c.flex == 0)
            .map(|(_, w)| *w)
            .sum();
        let flex: Vec<usize> = (0..self.cols.len()).filter(|i| self.cols[*i].flex > 0).collect();
        let available = total.saturating_sub(fixed + gaps);
        let wanted: usize = flex.iter().map(|i| natural[*i]).sum();
        if wanted <= available || flex.is_empty() {
            return widths;
        }
        // Minimum of a flex column: its `min`, else up to 4 columns (never more than it needs).
        let floor = |i: usize| self.cols[i].min.unwrap_or(4).min(natural[i]);
        let mut open: Vec<usize> = flex.clone();
        let mut left = available;
        // Columns that want less than their share keep their natural width; the rest split what remains.
        loop {
            let weight: usize = open.iter().map(|i| self.cols[*i].flex as usize).sum();
            if weight == 0 {
                break;
            }
            let mut settled = false;
            for &i in &open.clone() {
                let share = left * self.cols[i].flex as usize / weight;
                if natural[i] <= share {
                    widths[i] = natural[i];
                    left = left.saturating_sub(natural[i]);
                    open.retain(|j| *j != i);
                    settled = true;
                }
            }
            if !settled {
                let mut given = 0;
                for (n, &i) in open.iter().enumerate() {
                    let share = if n + 1 == open.len() {
                        left.saturating_sub(given)
                    } else {
                        left * self.cols[i].flex as usize / weight
                    };
                    widths[i] = share.max(floor(i));
                    given += share;
                }
                break;
            }
        }
        widths
    }
}

impl Render for Table {
    fn render(&self, mode: &Mode) -> String {
        if self.rows.is_empty() {
            return paint(mode, Token::Muted, &self.empty);
        }
        let widths = self.widths(mode.width);
        let line = |cells: Vec<String>| -> String {
            let joined = cells.join(&" ".repeat(GAP));
            joined.trim_end().to_string()
        };
        let mut out = Vec::with_capacity(self.rows.len() + 2);
        let header: Vec<String> = self
            .cols
            .iter()
            .zip(&widths)
            .map(|(c, w)| {
                let text = pad(&c.header, *w, c.align);
                paint(mode, Token::Ink, &text)
            })
            .collect();
        out.push(line(header));
        if !mode.plain {
            let total = widths.iter().sum::<usize>() + GAP * widths.len().saturating_sub(1);
            out.push(paint(mode, Token::Muted, &"─".repeat(total.min(mode.width))));
        }
        for row in &self.rows {
            let cells: Vec<String> = row
                .iter()
                .zip(&self.cols)
                .zip(&widths)
                .map(|((cell, col), w)| {
                    let text = pad(&truncate(&cell.text, *w), *w, col.align);
                    match cell.token {
                        Some(token) => paint(mode, token, &text),
                        None => text,
                    }
                })
                .collect();
            out.push(line(cells));
        }
        out.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::ColorLevel;

    fn table() -> Table {
        Table::new(&[
            Col::new("KEY"),
            Col::new("TITLE").flex(1),
            Col::new("EST").right(),
        ])
        .row(vec!["AUR-1".into(), "Short".into(), "3".into()])
        .row(vec!["AUR-12".into(), "x".repeat(150).into(), "13".into()])
    }

    #[test]
    fn fits_the_width() {
        for width in [40, 80, 120] {
            let mode = Mode::fixed(ColorLevel::None, false, width);
            let out = table().render(&mode);
            assert!(out.lines().all(|l| display_width(l) <= width), "{width}:\n{out}");
            assert!(out.contains('…'));
        }
    }

    #[test]
    fn natural_widths_when_room() {
        let t = Table::new(&[Col::new("A"), Col::new("B").flex(1)]).row(vec!["aa".into(), "bbb".into()]);
        assert_eq!(t.widths(80), vec![2, 3]);
        let mode = Mode::fixed(ColorLevel::None, true, 80);
        assert_eq!(t.render(&mode), "A   B\naa  bbb");
    }

    #[test]
    fn empty_text() {
        let mode = Mode::fixed(ColorLevel::None, false, 80);
        let t = Table::new(&[Col::new("A")]).empty_text("No tasks.");
        assert_eq!(t.render(&mode), "No tasks.");
    }
}
