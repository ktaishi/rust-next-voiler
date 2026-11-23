use serde::Serialize;
use uuid::Uuid;

/// カーソル (keyset) ページネーションの結果。
///
/// `next_cursor` は次ページ取得時に `cursor` クエリへ渡す不透明な値。
/// 実体は最後の要素の UUID v7 で、ID 順 = 作成順になる。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<Uuid>,
}

impl<T> Page<T> {
    /// `limit + 1` 件取得した結果から、次ページの有無を判定してページを組み立てる。
    pub fn from_overfetch(mut rows: Vec<T>, limit: usize, id_of: impl Fn(&T) -> Uuid) -> Self {
        let has_more = rows.len() > limit;
        rows.truncate(limit);
        let next_cursor = if has_more {
            rows.last().map(id_of)
        } else {
            None
        };
        Self {
            items: rows,
            next_cursor,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_cursor_is_set_only_when_more_rows_exist() {
        let ids: Vec<Uuid> = (0..3).map(|_| Uuid::now_v7()).collect();
        let p = Page::from_overfetch(ids.clone(), 2, |u| *u);
        assert_eq!(p.items.len(), 2);
        assert_eq!(p.next_cursor, Some(ids[1]));

        let p = Page::from_overfetch(ids[..2].to_vec(), 2, |u| *u);
        assert_eq!(p.next_cursor, None);
    }
}
