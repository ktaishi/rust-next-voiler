/// MySQL のプリペアドステートメント 1 文あたりのプレースホルダ上限。
pub const MYSQL_MAX_PLACEHOLDERS: usize = 65_535;

/// バルク INSERT 1 文に含める行数。要求値を、プレースホルダ上限に収まる範囲へ丸める。
pub fn rows_per_statement(requested: usize, columns_per_row: usize) -> usize {
    assert!(columns_per_row > 0, "columns_per_row must be positive");
    requested.clamp(1, MYSQL_MAX_PLACEHOLDERS / columns_per_row)
}

#[cfg(test)]
mod tests {
    use super::*;

    // AC-7
    #[test]
    fn rows_per_statement_respects_placeholder_limit() {
        assert_eq!(rows_per_statement(1_000, 5), 1_000);
        assert_eq!(rows_per_statement(100_000, 5), 13_107);
        assert!(rows_per_statement(100_000, 5) * 5 <= MYSQL_MAX_PLACEHOLDERS);
        assert_eq!(rows_per_statement(0, 5), 1);
    }
}
