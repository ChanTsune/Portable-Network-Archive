use ansi_term::{ANSIString, ANSIStrings, Style};
use std::vec::IntoIter;

pub(crate) struct Table<const N: usize> {
    rows: Vec<TableRow<N>>,
}

impl<const N: usize> Table<N> {
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
                max_widths[i] = max_widths[i].max(col.text.iter().map(|i| i.len()).sum());
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
        Some(
            row.render(&self.max_widths)
                .into_iter()
                .map(|i| ANSIStrings(&i).to_string())
                .collect::<Vec<_>>()
                .join(" "),
        )
    }
}

pub(crate) struct TableRow<const N: usize> {
    columns: [Cell; N],
}

pub(crate) fn header(style: Style) -> TableRow<10> {
    TableRow::new([
        Cell::new(style, "Encryption"),
        Cell::new(style, "Compression"),
        Cell::new(style, "Permissions"),
        Cell::new(style, "Raw Size"),
        Cell::new(style, "Compressed Size"),
        Cell::new(style, "User"),
        Cell::new(style, "Group"),
        Cell::new(style, "Created"),
        Cell::new(style, "Modified"),
        Cell::new(style, "Name"),
    ])
}

impl<const N: usize> TableRow<N> {
    pub(crate) const fn new(columns: [Cell; N]) -> Self {
        Self { columns }
    }

    pub(crate) fn render(&self, max_widths: &[usize; N]) -> Vec<Vec<ANSIString>> {
        self.columns
            .iter()
            .zip(max_widths)
            .map(|(c, m)| c.render(*m))
            .collect::<Vec<_>>()
    }
}

pub(crate) enum Padding {
    Left,
    Right,
}

pub(crate) struct Cell {
    style: Style,
    text: Vec<ANSIString<'static>>,
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
            text: vec![style.paint(text.to_string())],
        }
    }
    pub(crate) fn new_text(
        style: Style,
        pad_direction: Padding,
        text: Vec<ANSIString<'static>>,
    ) -> Self {
        Self {
            style,
            text,
            pad_direction,
        }
    }

    pub(crate) fn render(&self, max_width: usize) -> Vec<ANSIString> {
        let len: usize = self.text.iter().map(|i| i.len()).sum();
        let p = " ".repeat(max_width - len);
        let mut result = vec![];
        match self.pad_direction {
            Padding::Left => {
                result.push(self.style.paint(p));
                result.extend(self.text.clone());
            }
            Padding::Right => {
                result.extend(self.text.clone());
                result.push(self.style.paint(p));
            }
        }
        result
    }
}
