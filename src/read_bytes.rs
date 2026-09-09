use std::io::{self, Read};

type Error = crate::NcwError;

/// Extensions to io::Read for simplifying reading bytes.
pub trait ReadBytesExt: Read {
    fn read_u16_le(&mut self) -> io::Result<u16> {
        let mut buf = [0u8; 2];
        self.read_exact(&mut buf)?;
        Ok(u16::from_le_bytes(buf))
    }

    fn read_i16_le(&mut self) -> io::Result<i16> {
        let mut buf = [0u8; 2];
        self.read_exact(&mut buf)?;
        Ok(i16::from_le_bytes(buf))
    }

    fn read_u32_le(&mut self) -> io::Result<u32> {
        let mut buf = [0u8; 4];
        self.read_exact(&mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }

    fn read_i32_le(&mut self) -> io::Result<i32> {
        let mut buf = [0u8; 4];
        self.read_exact(&mut buf)?;
        Ok(i32::from_le_bytes(buf))
    }

    fn read_u32_be(&mut self) -> io::Result<u32> {
        let mut buf = [0u8; 4];
        self.read_exact(&mut buf)?;
        Ok(u32::from_be_bytes(buf))
    }

    fn read_u64_be(&mut self) -> io::Result<u64> {
        let mut buf = [0u8; 8];
        self.read_exact(&mut buf)?;
        Ok(u64::from_be_bytes(buf))
    }

    /// Read exactly `bytes` bytes, or fail with `NcwError::ReadError`.
    fn read_bytes(&mut self, bytes: usize) -> Result<Vec<u8>, Error> {
        let mut buf = vec![0u8; bytes];
        self.read_exact(&mut buf)
            .map_err(|_| Error::ReadError(bytes))?;
        Ok(buf)
    }
}

impl<R: Read + ?Sized> ReadBytesExt for R {}

#[cfg(test)]
mod tests {
    use super::ReadBytesExt;
    use std::io;

    #[test]
    fn test_read_u32_le() {
        let bytes: &[u8] = &[32_u8, 1, 4, 56, 6, 6, 90, 4, 7];
        let mut cursor = io::Cursor::new(bytes);

        let num = cursor.read_u32_le().unwrap();
        assert_eq!(num, 939786528);

        let num = cursor.read_u32_le().unwrap();
        assert_eq!(num, 73008646);
    }

    #[test]
    fn test_read_bytes_short() {
        let mut cursor = io::Cursor::new(&[1u8, 2][..]);
        assert!(matches!(
            cursor.read_bytes(4),
            Err(crate::NcwError::ReadError(4))
        ));
    }
}
