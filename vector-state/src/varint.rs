use std::io;

pub trait Varint: Sized {
    fn size(&self) -> u8;
    fn serialize<W: io::Write>(&self, writer: &mut W) -> io::Result<()>;
    fn deserialize<R: io::Read>(reader: &mut R) -> io::Result<Self>;
}

macro_rules! impl_unsigned_varint {
    (u64) => (impl_unsigned_varint!(@impl u64, size_64, i64););
    ($t:ty) => (impl_unsigned_varint!(@impl $t, size_32, i32););

    (@impl $t:ty, $size_fn:ident, $size_t:ty) => {
        impl Varint for $t {
            #[inline]
            fn size(&self) -> u8 {
                $size_fn(*self as $size_t)
            }

            fn serialize<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
                let mut value = *self;

                loop {
                    if value & !0x7f == 0 {
                        writer.write(&[value as u8])?;
                        return Ok(());
                    } else {
                        writer.write(&[value as u8 & 0x7f | 0x80])?;
                        value >>= 7;
                    }
                }
            }

            fn deserialize<R: io::Read>(reader: &mut R) -> io::Result<Self> {
                let mut value = 0;

                let mut buf = [0];
                let mut offset = 0;

                loop {
                    reader.read_exact(&mut buf)?;
                    value |= (buf[0] & 0x7f) as $t << offset;

                    if buf[0] & 0x80 == 0 {
                        return Ok(value);
                    }

                    offset += 7;
                }
            }
        }
    };
}

impl_unsigned_varint!(u8);
impl_unsigned_varint!(u16);
impl_unsigned_varint!(u32);
impl_unsigned_varint!(u64);

macro_rules! impl_signed_varint {
    ($t:ty, $ut:ty) => {
        impl Varint for $t {
            #[inline]
            fn size(&self) -> u8 {
                (*self as $ut).size()
            }

            fn serialize<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
                use std::mem::size_of;
                // ZigZag encoding: https://bit.ly/2Pl9Gq8
                let encoded = ((self << 1) ^ (self >> ((size_of::<$t>() << 3) - 1))) as $ut;
                encoded.serialize(writer)
            }

            fn deserialize<R: io::Read>(reader: &mut R) -> io::Result<Self> {
                let encoded = <$ut>::deserialize(reader)? as $t;
                Ok((encoded >> 1) ^ -(encoded & 1))
            }
        }
    };
}

impl_signed_varint!(i8, u8);
impl_signed_varint!(i16, u16);
impl_signed_varint!(i32, u32);
impl_signed_varint!(i64, u64);

// Reference: https://bit.ly/2BJbkd5
pub fn size_32(value: i32) -> u8 {
    if value & (!0 << 7) == 0 {
        return 1;
    }

    if value & (!0 << 14) == 0 {
        return 2;
    }

    if value & (!0 << 21) == 0 {
        return 3;
    }

    if value & (!0 << 28) == 0 {
        return 4;
    }

    5
}

// Reference: https://bit.ly/2MPq54D
pub fn size_64(mut value: i64) -> u8 {
    // Handle two popular special cases upfront ...
    if value & (!0i64 << 7) == 0 {
        return 1;
    }

    if value < 0 {
        return 10;
    }

    // ... leaving us with 8 remaining, which we can divide and conquer
    let mut size = 2;

    if value & (!0i64 << 35) != 0 {
        size += 4;
        value >>= 28;
    }

    if value & (!0i64 << 21) != 0 {
        size += 2;
        value >>= 14;
    }

    if value & (!0i64 << 14) != 0 {
        size += 1;
    }

    size
}
