use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeError {
    pub offset: usize,
    pub reason: &'static str,
}

impl DecodeError {
    pub(crate) const fn new(offset: usize, reason: &'static str) -> Self {
        Self { offset, reason }
    }
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at byte {:#x}", self.reason, self.offset)
    }
}
impl std::error::Error for DecodeError {}

#[derive(Clone, Copy)]
pub(crate) struct Reader<'a>(pub &'a [u8]);

impl<'a> Reader<'a> {
    pub fn bytes(self, offset: usize, length: usize) -> Result<&'a [u8], DecodeError> {
        let end = offset
            .checked_add(length)
            .ok_or_else(|| DecodeError::new(offset, "extent overflow"))?;
        self.0
            .get(offset..end)
            .ok_or_else(|| DecodeError::new(offset, "truncated data"))
    }
    pub fn u8(self, at: usize) -> Result<u8, DecodeError> {
        Ok(self.bytes(at, 1)?[0])
    }
    pub fn u16(self, at: usize) -> Result<u16, DecodeError> {
        Ok(u16::from_le_bytes(self.bytes(at, 2)?.try_into().unwrap()))
    }
    pub fn i16(self, at: usize) -> Result<i16, DecodeError> {
        Ok(self.u16(at)? as i16)
    }
    pub fn u32(self, at: usize) -> Result<u32, DecodeError> {
        Ok(u32::from_le_bytes(self.bytes(at, 4)?.try_into().unwrap()))
    }
    pub fn i32(self, at: usize) -> Result<i32, DecodeError> {
        Ok(self.u32(at)? as i32)
    }
    pub fn array(self, at: usize, count: usize, stride: usize) -> Result<&'a [u8], DecodeError> {
        let length = count
            .checked_mul(stride)
            .ok_or_else(|| DecodeError::new(at, "array size overflow"))?;
        self.bytes(at, length)
    }
}
