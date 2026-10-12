//! Turns stored values into the text a grid cell shows, and sizes the columns. No drawing.

use nocodo_types::outcome::{CellValue, RowColumn};
use nocodo_types::schema::ColumnType;

const MIN_WIDTH: f32 = 70.0;
const MAX_WIDTH: f32 = 260.0;
const CHAR_WIDTH: f32 = 8.0;
const CELL_PAD: f32 = 28.0;

/// Money is stored in minor units (cents); show it as `12.50`.
fn money(units: i64) -> String {
    let sign = if units < 0 { "-" } else { "" };
    let abs = units.unsigned_abs();
    format!("{sign}{}.{:02}", abs / 100, abs % 100)
}

/// The text of one cell. A value that does not fit its column type is shown as it is.
pub fn cell_text(column_type: ColumnType, value: &CellValue) -> String {
    match value {
        CellValue::Null => String::new(),
        CellValue::Bool(true) => "Yes".into(),
        CellValue::Bool(false) => "No".into(),
        CellValue::Integer(n) if column_type == ColumnType::Money => money(*n),
        CellValue::Integer(n) => n.to_string(),
        CellValue::Real(x) => {
            let text = format!("{x:.4}");
            text.trim_end_matches('0').trim_end_matches('.').to_string()
        }
        CellValue::Text(t) => t.clone(),
    }
}

/// Column widths in pixels, from the heading and the longest of the shown values.
pub fn column_widths(columns: &[RowColumn], rows: &[Vec<CellValue>]) -> Vec<f32> {
    columns
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let longest = rows
                .iter()
                .filter_map(|r| r.get(i))
                .map(|v| cell_text(c.column_type, v).chars().count())
                .chain(std::iter::once(c.label.chars().count()))
                .max()
                .unwrap_or(0);
            (longest as f32 * CHAR_WIDTH + CELL_PAD).clamp(MIN_WIDTH, MAX_WIDTH)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn col(label: &str, t: ColumnType) -> RowColumn {
        RowColumn {
            name: label.into(),
            label: label.into(),
            column_type: t,
        }
    }

    #[test]
    fn values_are_shown_in_plain_form() {
        assert_eq!(
            cell_text(ColumnType::Money, &CellValue::Integer(1250)),
            "12.50"
        );
        assert_eq!(
            cell_text(ColumnType::Money, &CellValue::Integer(-5)),
            "-0.05"
        );
        assert_eq!(cell_text(ColumnType::Integer, &CellValue::Integer(7)), "7");
        assert_eq!(
            cell_text(ColumnType::Boolean, &CellValue::Bool(true)),
            "Yes"
        );
        assert_eq!(
            cell_text(ColumnType::Boolean, &CellValue::Bool(false)),
            "No"
        );
        assert_eq!(cell_text(ColumnType::Text, &CellValue::Null), "");
        assert_eq!(cell_text(ColumnType::Decimal, &CellValue::Real(2.5)), "2.5");
        assert_eq!(cell_text(ColumnType::Decimal, &CellValue::Real(3.0)), "3");
        assert_eq!(
            cell_text(ColumnType::Date, &CellValue::Text("2026-10-01".into())),
            "2026-10-01"
        );
        // A stray text value in a money column is shown, not hidden.
        assert_eq!(
            cell_text(ColumnType::Money, &CellValue::Text("n/a".into())),
            "n/a"
        );
    }

    #[test]
    fn widths_follow_content_within_limits() {
        let columns = vec![
            col("id", ColumnType::Integer),
            col("note", ColumnType::Text),
        ];
        let rows = vec![vec![
            CellValue::Integer(1),
            CellValue::Text("x".repeat(100)),
        ]];
        let w = column_widths(&columns, &rows);
        assert_eq!(w[0], MIN_WIDTH);
        assert_eq!(w[1], MAX_WIDTH);
        assert_eq!(column_widths(&columns, &[]).len(), 2);
    }
}
