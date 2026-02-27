use mongodb::bson::{Bson, Document};

pub(super) fn nested_document<'a>(document: &'a Document, path: &[&str]) -> Option<&'a Document> {
    let mut current = document;

    for key in path {
        current = current.get_document(*key).ok()?;
    }

    Some(current)
}

pub(super) fn nested_i64(document: &Document, path: &[&str]) -> Option<i64> {
    if path.is_empty() {
        return None;
    }

    let parent = if path.len() == 1 {
        Some(document)
    } else {
        nested_document(document, &path[..path.len() - 1])
    }?;

    parent.get(path[path.len() - 1]).and_then(bson_to_i64)
}

pub(super) fn nested_f64(document: &Document, path: &[&str]) -> Option<f64> {
    if path.is_empty() {
        return None;
    }

    let parent = if path.len() == 1 {
        Some(document)
    } else {
        nested_document(document, &path[..path.len() - 1])
    }?;

    parent.get(path[path.len() - 1]).and_then(bson_to_f64)
}

pub(super) fn nested_bool(document: &Document, path: &[&str]) -> Option<bool> {
    if path.is_empty() {
        return None;
    }

    let parent = if path.len() == 1 {
        Some(document)
    } else {
        nested_document(document, &path[..path.len() - 1])
    }?;

    parent
        .get(path[path.len() - 1])
        .and_then(|value| match value {
            Bson::Boolean(value) => Some(*value),
            _ => None,
        })
}

pub(super) fn nested_string(document: &Document, path: &[&str]) -> Option<String> {
    if path.is_empty() {
        return None;
    }

    let parent = if path.len() == 1 {
        Some(document)
    } else {
        nested_document(document, &path[..path.len() - 1])
    }?;

    parent
        .get(path[path.len() - 1])
        .and_then(|value| match value {
            Bson::String(value) => Some(value.clone()),
            _ => None,
        })
}

pub(super) fn nested_array<'a>(document: &'a Document, path: &[&str]) -> Option<&'a Vec<Bson>> {
    if path.is_empty() {
        return None;
    }

    let parent = if path.len() == 1 {
        Some(document)
    } else {
        nested_document(document, &path[..path.len() - 1])
    }?;

    parent
        .get(path[path.len() - 1])
        .and_then(|value| value.as_array())
}

fn bson_to_i64(value: &Bson) -> Option<i64> {
    match value {
        Bson::Int32(value) => Some(i64::from(*value)),
        Bson::Int64(value) => Some(*value),
        Bson::Double(value) => {
            if value.is_finite() {
                Some(*value as i64)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn bson_to_f64(value: &Bson) -> Option<f64> {
    match value {
        Bson::Int32(value) => Some(f64::from(*value)),
        Bson::Int64(value) => Some(*value as f64),
        Bson::Double(value) => {
            if value.is_finite() {
                Some(*value)
            } else {
                None
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use mongodb::bson::Bson;

    use super::{bson_to_f64, bson_to_i64};

    #[test]
    fn converts_numeric_bson_to_i64() {
        assert_eq!(bson_to_i64(&Bson::Int32(12)), Some(12));
        assert_eq!(bson_to_i64(&Bson::Int64(34)), Some(34));
        assert_eq!(bson_to_i64(&Bson::Double(56.8)), Some(56));
        assert_eq!(bson_to_i64(&Bson::Double(f64::INFINITY)), None);
        assert_eq!(bson_to_i64(&Bson::Null), None);
    }

    #[test]
    fn converts_numeric_bson_to_f64() {
        assert_eq!(bson_to_f64(&Bson::Int32(12)), Some(12.0));
        assert_eq!(bson_to_f64(&Bson::Int64(34)), Some(34.0));
        assert_eq!(bson_to_f64(&Bson::Double(56.8)), Some(56.8));
        assert_eq!(bson_to_f64(&Bson::Double(f64::NAN)), None);
        assert_eq!(bson_to_f64(&Bson::Null), None);
    }
}
