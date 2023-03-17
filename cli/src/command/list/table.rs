use ansi_term::Style;
use std::vec::IntoIter;

pub(crate) struct Table {
    rows: Vec<TableRow>,
}

impl Table {
    pub(crate) fn new() -> Self {
        Self {
            rows: Default::default(),
        }
    }

    pub(crate) fn push(&mut self, row: TableRow) {
        self.rows.push(row)
    }

    pub(crate) fn into_render_rows(self) -> TableIter {
        let mut max_widths = vec![0; self.rows.first().map_or(0, |r| r.columns.len())];
        for row in &self.rows {
            for (i, col) in row.columns.iter().enumerate() {
                max_widths[i] = max_widths[i].max(col.text.len());
            }
        }
        TableIter {
            max_widths,
            iter: self.rows.into_iter(),
        }
    }
}

pub(crate) struct TableIter {
    max_widths: Vec<usize>,
    iter: IntoIter<TableRow>,
}

impl Iterator for TableIter {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        let row = self.iter.next()?;
        Some(row.render(&self.max_widths))
    }
}

pub(crate) struct TableRow {
    columns: Vec<Cell>,
}

impl TableRow {
    pub(crate) fn new(columns: Vec<Cell>) -> Self {
        Self { columns }
    }

    pub(crate) fn header(style: Style) -> Self {
        Self::new(vec![
            Cell::new(style, "Encryption"),
            Cell::new(style, "Compression"),
            Cell::new(style, "Compressed Size"),
            Cell::new(style, "Name"),
        ])
    }

    pub(crate) fn render(&self, max_widths: &[usize]) -> String {
        self.columns
            .iter()
            .zip(max_widths)
            .map(|(c, m)| c.render(*m))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

pub(crate) enum Padding {
    Left,
    Right,
}

pub(crate) struct Cell {
    style: Style,
    text: String,
    pad_direction: Padding,
}

impl Cell {
    pub(crate) fn new<S: ToString>(style: Style, text: S) -> Self {
        Self::new_with_pad_direction(style, Padding::Right, text)
    }

    pub(crate) fn new_with_pad_direction<S: ToString>(
        style: Style,
        pad_direction: Padding,
        text: S,
    ) -> Self {
        Self {
            style,
            pad_direction,
            text: text.to_string(),
        }
    }

    pub(crate) fn render(&self, max_width: usize) -> String {
        self.style
            .paint(match self.pad_direction {
                Padding::Left => format!("{:>width$}", self.text, width = max_width),
                Padding::Right => format!("{:<width$}", self.text, width = max_width),
            })
            .to_string()
    }
}
