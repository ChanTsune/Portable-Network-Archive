use ansi_term::Style;
use std::vec::IntoIter;

pub(crate) struct Table<const N: usize> {
    rows: Vec<TableRow<N>>,
}

impl<const N: usize> Table<N> {
    pub(crate) fn new_with_header(header: TableRow<N>) -> Self {
        Self { rows: vec![header] }
    }

    pub(crate) fn new() -> Self {
        Self {
            rows: Default::default(),
        }
    }

    pub(crate) fn push(&mut self, row: TableRow<N>) {
        self.rows.push(row)
    }

    pub(crate) fn into_render_rows(self) -> TableIter<N> {
        let mut max_widths = [0; N];
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

pub(crate) struct TableIter<const N: usize> {
    max_widths: [usize; N],
    iter: IntoIter<TableRow<N>>,
}

impl<const N: usize> Iterator for TableIter<N> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        let row = self.iter.next()?;
        Some(row.render(&self.max_widths))
    }
}

pub(crate) struct TableRow<const N: usize> {
    columns: [Cell; N],
}

pub(crate) fn header(style: Style) -> TableRow<6> {
    TableRow::new([
        Cell::new(style, "Encryption"),
        Cell::new(style, "Compression"),
        Cell::new(style, "Compressed Size"),
        Cell::new(style, "Created"),
        Cell::new(style, "Modified"),
        Cell::new(style, "Name"),
    ])
}

impl<const N: usize> TableRow<N> {
    pub(crate) fn new(columns: [Cell; N]) -> Self {
        Self { columns }
    }

    pub(crate) fn render(&self, max_widths: &[usize; N]) -> String {
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
