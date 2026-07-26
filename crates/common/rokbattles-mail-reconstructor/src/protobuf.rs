use crate::ReconstructionError;

#[derive(Debug, Clone, Copy)]
pub(crate) enum FieldValue<'a> {
    Varint(u64),
    Bytes(&'a [u8]),
    Fixed64(u64),
    Fixed32(u32),
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Field<'a> {
    pub number: u32,
    pub value: FieldValue<'a>,
}

pub(crate) fn fields(data: &[u8]) -> FieldCursor<'_> {
    FieldCursor { data, position: 0 }
}

pub(crate) struct FieldCursor<'a> {
    data: &'a [u8],
    position: usize,
}

impl<'a> Iterator for FieldCursor<'a> {
    type Item = Result<Field<'a>, ReconstructionError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position == self.data.len() {
            return None;
        }
        Some(self.read_field())
    }
}

impl<'a> FieldCursor<'a> {
    fn read_field(&mut self) -> Result<Field<'a>, ReconstructionError> {
        let tag = self.read_varint()?;
        let number =
            u32::try_from(tag >> 3).map_err(|_error| ReconstructionError::IntegerOutOfRange)?;
        if number == 0 {
            return Err(ReconstructionError::InvalidProtobuf("field number cannot be zero"));
        }
        let value = match tag & 0x07 {
            0 => FieldValue::Varint(self.read_varint()?),
            1 => FieldValue::Fixed64(u64::from_le_bytes(self.read_array()?)),
            2 => FieldValue::Bytes(self.read_bytes()?),
            5 => FieldValue::Fixed32(u32::from_le_bytes(self.read_array()?)),
            _ => {
                return Err(ReconstructionError::InvalidProtobuf(
                    "protobuf wire type is unsupported",
                ));
            }
        };
        Ok(Field { number, value })
    }

    fn read_varint(&mut self) -> Result<u64, ReconstructionError> {
        let mut value = 0u64;
        for shift in (0..64).step_by(7) {
            let byte = *self
                .data
                .get(self.position)
                .ok_or(ReconstructionError::InvalidProtobuf("truncated protobuf varint"))?;
            self.position = self.position.saturating_add(1);
            value |= u64::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err(ReconstructionError::InvalidProtobuf("protobuf varint exceeded 64 bits"))
    }

    fn read_bytes(&mut self) -> Result<&'a [u8], ReconstructionError> {
        let length = usize::try_from(self.read_varint()?)
            .map_err(|_error| ReconstructionError::IntegerOutOfRange)?;
        let end = self
            .position
            .checked_add(length)
            .ok_or(ReconstructionError::InvalidProtobuf("protobuf field length overflowed"))?;
        let bytes = self
            .data
            .get(self.position..end)
            .ok_or(ReconstructionError::InvalidProtobuf("truncated protobuf field"))?;
        self.position = end;
        Ok(bytes)
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], ReconstructionError> {
        let end = self
            .position
            .checked_add(N)
            .ok_or(ReconstructionError::InvalidProtobuf("protobuf field length overflowed"))?;
        let bytes = self
            .data
            .get(self.position..end)
            .ok_or(ReconstructionError::InvalidProtobuf("truncated protobuf field"))?;
        self.position = end;
        bytes
            .try_into()
            .map_err(|_error| ReconstructionError::InvalidProtobuf("truncated protobuf field"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_each_supported_wire_type() {
        let mut encoded = vec![0x08, 0x96, 0x01, 0x11];
        encoded.extend_from_slice(&42_u64.to_le_bytes());
        encoded.extend_from_slice(&[0x1a, 0x03, b'm', b'a', b'i', 0x25]);
        encoded.extend_from_slice(&7_u32.to_le_bytes());

        let parsed = fields(&encoded).collect::<Result<Vec<_>, _>>().expect("fields should parse");

        assert_eq!(parsed.len(), 4);
        assert!(matches!(parsed[0], Field { number: 1, value: FieldValue::Varint(150) }));
        assert!(matches!(parsed[1], Field { number: 2, value: FieldValue::Fixed64(42) }));
        assert!(matches!(parsed[2], Field { number: 3, value: FieldValue::Bytes(b"mai") }));
        assert!(matches!(parsed[3], Field { number: 4, value: FieldValue::Fixed32(7) }));
    }

    #[test]
    fn rejects_zero_field_number_and_unsupported_wire_type() {
        assert!(matches!(
            fields(&[0]).next(),
            Some(Err(ReconstructionError::InvalidProtobuf("field number cannot be zero")))
        ));
        assert!(matches!(
            fields(&[0x0b]).next(),
            Some(Err(ReconstructionError::InvalidProtobuf("protobuf wire type is unsupported")))
        ));
    }

    #[test]
    fn rejects_truncated_length_delimited_and_fixed_fields() {
        assert!(matches!(
            fields(&[0x0a, 0x02, 0x01]).next(),
            Some(Err(ReconstructionError::InvalidProtobuf("truncated protobuf field")))
        ));
        assert!(matches!(
            fields(&[0x09, 0x01]).next(),
            Some(Err(ReconstructionError::InvalidProtobuf("truncated protobuf field")))
        ));
        assert!(matches!(
            fields(&[0x15, 0x01]).next(),
            Some(Err(ReconstructionError::InvalidProtobuf("truncated protobuf field")))
        ));
    }

    #[test]
    fn rejects_truncated_and_oversized_varints() {
        assert!(matches!(
            fields(&[0x08, 0x80]).next(),
            Some(Err(ReconstructionError::InvalidProtobuf("truncated protobuf varint")))
        ));

        let mut encoded = vec![0x08];
        encoded.extend_from_slice(&[0x80; 10]);
        assert!(matches!(
            fields(&encoded).next(),
            Some(Err(ReconstructionError::InvalidProtobuf("protobuf varint exceeded 64 bits")))
        ));
    }
}
