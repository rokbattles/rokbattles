use mongodb::bson::Bson;

/// Convert BSON number types we expect into i64.
pub(super) fn bson_to_i64(value: &Bson) -> Option<i64> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_bson_number_to_i64() {
        assert_eq!(bson_to_i64(&Bson::Int32(12)), Some(12));
        assert_eq!(bson_to_i64(&Bson::Int64(34)), Some(34));
        assert_eq!(bson_to_i64(&Bson::Double(56.8)), Some(56));
        assert_eq!(bson_to_i64(&Bson::Null), None);
    }
}
