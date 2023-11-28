use sqlx::{sqlite::SqliteRow, Column, Row};

pub struct ResultSet {
    rows: Vec<SqliteRow>,
}

pub fn row_to_string(row: &SqliteRow) -> Vec<String> {
    iter_row(row)
        .map(|v| String::from_utf8_lossy(v).to_string())
        .collect()
}

pub fn iter_row(row: &SqliteRow) -> impl Iterator<Item = &[u8]> {
    row.columns()
        .iter()
        .filter_map(|c| row.try_get_unchecked(c.ordinal()).ok())
}

impl ResultSet {
    pub fn new(rows: Vec<SqliteRow>) -> Self {
        Self { rows }
    }

    #[allow(dead_code)]
    pub fn print(&self) {
        self.rows.iter().for_each(|r| {
            for c in iter_row(r) {
                dbg!(c);
            }
        });
    }

    #[allow(dead_code)]
    pub fn to_csv(self) {
        let mut csv = String::new();

        self.rows.iter().for_each(|r| {
            let values: Vec<String> = row_to_string(r);

            let row = values.join(",");
            csv.push_str(&row);
            csv.push('\n');
        });

        println!("{}", csv);
    }

    pub fn to_csv_rows(&self) -> Vec<String> {
        self.rows
            .iter()
            .map(|r| {
                let values: Vec<String> = row_to_string(r);
                values.join(",")
            })
            .collect()
    }
}
