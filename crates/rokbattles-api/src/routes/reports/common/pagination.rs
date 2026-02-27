/// Cursor paging result shared by list endpoints.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct CursorPage<T> {
    pub items: Vec<T>,
    pub next_after: Option<String>,
    pub previous_before: Option<String>,
}

/// Build one page from rows fetched in query order.
///
/// `before_cursor` means the DB query ran ascending, so we reverse before returning.
pub(crate) fn paginate_cursor_rows<T, F>(
    rows: Vec<T>,
    fetched_documents: usize,
    page_size: usize,
    before_cursor: Option<i64>,
    after_cursor: Option<i64>,
    cursor_value: F,
) -> CursorPage<T>
where
    F: Fn(&T) -> i64,
{
    let has_more_in_query_direction = fetched_documents > page_size;
    let paged_rows = if has_more_in_query_direction {
        rows.into_iter().take(page_size).collect::<Vec<_>>()
    } else {
        rows
    };

    let ordered_rows = if before_cursor.is_some() {
        paged_rows.into_iter().rev().collect::<Vec<_>>()
    } else {
        paged_rows
    };

    let first_row = ordered_rows.first();
    let last_row = ordered_rows.last();
    let is_initial_page = before_cursor.is_none() && after_cursor.is_none();

    let previous_before = if let Some(first_row) = first_row {
        if !is_initial_page
            && (after_cursor.is_some() || (before_cursor.is_some() && has_more_in_query_direction))
        {
            Some(cursor_value(first_row).to_string())
        } else {
            None
        }
    } else {
        None
    };

    let next_after = if let Some(last_row) = last_row {
        if before_cursor.is_some() || has_more_in_query_direction {
            Some(cursor_value(last_row).to_string())
        } else {
            None
        }
    } else {
        None
    };

    CursorPage {
        items: ordered_rows,
        next_after,
        previous_before,
    }
}

#[cfg(test)]
mod tests {
    use super::{CursorPage, paginate_cursor_rows};

    #[test]
    fn paginates_initial_page_without_cursors() {
        let rows = vec![10_i64, 9, 8];

        let page = paginate_cursor_rows(rows, 3, 3, None, None, |value| *value);

        assert_eq!(
            page,
            CursorPage {
                items: vec![10, 9, 8],
                next_after: None,
                previous_before: None,
            }
        );
    }

    #[test]
    fn paginates_forward_page_with_next_and_previous_tokens() {
        let rows = vec![7_i64, 6, 5, 4];

        let page = paginate_cursor_rows(rows, 4, 3, None, Some(8), |value| *value);

        assert_eq!(
            page,
            CursorPage {
                items: vec![7, 6, 5],
                next_after: Some("5".to_string()),
                previous_before: Some("7".to_string()),
            }
        );
    }

    #[test]
    fn paginates_before_cursor_and_restores_descending_order() {
        let rows = vec![4_i64, 5, 6, 7];

        let page = paginate_cursor_rows(rows, 4, 3, Some(8), None, |value| *value);

        assert_eq!(
            page,
            CursorPage {
                items: vec![6, 5, 4],
                next_after: Some("4".to_string()),
                previous_before: Some("6".to_string()),
            }
        );
    }

    #[test]
    fn paginates_empty_result_without_tokens() {
        let page = paginate_cursor_rows(Vec::<i64>::new(), 0, 3, None, Some(10), |value| *value);

        assert_eq!(
            page,
            CursorPage {
                items: Vec::new(),
                next_after: None,
                previous_before: None,
            }
        );
    }
}
