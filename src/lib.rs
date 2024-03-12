#![feature(core_intrinsics)]
#![feature(const_type_name)]
// #![feature(specialization)]
use bit_vec::BitVec;
use std::intrinsics::type_name;
use std::io::{self, Read, Write};
use std::net::SocketAddr;
pub use write_to_derive::{Length, NormalizedIntegerAccessors, ReadFrom, WriteTo};

pub trait WriteTo {
    fn write_to<T: Write>(&self, writer: &mut T) -> io::Result<()>;
}

pub trait ReadFrom
where
    Self: Sized,
{
    // Return read thing and remaining length
    fn read_from<T: Read>(reader: &mut T, length: usize) -> io::Result<(Self, usize)>;
}

pub trait Length {
    fn length(&self) -> usize;

    fn length_be_bytes(&self) -> [u8; 4] {
        return (self.length() as u32).to_be_bytes();
    }
}

pub trait ConstLength: Length {}

pub trait Name {
    const NAME: &'static str;
}

impl<T> Name for T {
    const NAME: &'static str = type_name::<Self>();
}

// I could have a default impl instead, but I'd rather see the errors for types I haven't explicitly defined something for.
macro_rules! ImplPrimativeLength {
    ($NAME:ident) => {
        impl Length for $NAME {
            fn length(&self) -> usize {
                std::mem::size_of::<$NAME>()
            }
        }
    };
}
// ImplPrimativeLength!(u8);
ImplPrimativeLength!(u16);
ImplPrimativeLength!(u32);
ImplPrimativeLength!(u64);
ImplPrimativeLength!(i16);
ImplPrimativeLength!(i32);
ImplPrimativeLength!(i64);

impl<const N: usize> Length for [u8; N] {
    fn length(&self) -> usize {
        N
    }
}

impl Length for BitVec {
    fn length(&self) -> usize {
        // ceil(bits/8)
        (self.len() + 7) / 8
    }
}

impl<T: Length> Length for Vec<T> {
    fn length(&self) -> usize {
        // Optimize with FixedLength / Length?
        let mut result = 0;
        for val in self {
            result += val.length();
        }
        result
    }
}

impl Length for Vec<u8> {
    fn length(&self) -> usize {
        self.len()
    }
}

macro_rules! ImplWriteToPrimative {
    ($NAME:ident) => {
        impl WriteTo for $NAME {
            fn write_to<T: Write>(&self, writer: &mut T) -> io::Result<()> {
                writer.write_all(&self.to_be_bytes())?;
                Ok(())
            }
        }
    };
}

// ImplWriteToPrimative!(u8);
ImplWriteToPrimative!(u16);
ImplWriteToPrimative!(u32);
ImplWriteToPrimative!(u64);
ImplWriteToPrimative!(i16);
ImplWriteToPrimative!(i32);
ImplWriteToPrimative!(i64);

impl<W: WriteTo> WriteTo for std::vec::Vec<W> {
    fn write_to<T: Write>(&self, writer: &mut T) -> io::Result<()> {
        for val in self {
            val.write_to(writer)?;
        }
        Ok(())
    }
}

impl WriteTo for std::vec::Vec<u8> {
    fn write_to<T: Write>(&self, writer: &mut T) -> io::Result<()> {
        writer.write_all(&self)
    }
}

impl<const N: usize> WriteTo for [u8; N] {
    fn write_to<T: Write>(&self, writer: &mut T) -> io::Result<()> {
        writer.write_all(self)
    }
}

impl WriteTo for BitVec {
    fn write_to<T: Write>(&self, writer: &mut T) -> std::io::Result<()> {
        let bytes = self.to_bytes();
        writer.write_all(&bytes)?;
        Ok(())
    }
}

impl WriteTo for String {
    fn write_to<T: Write>(&self, writer: &mut T) -> std::io::Result<()> {
        let bytes = self.as_bytes();
        writer.write_all(&bytes)?;
        Ok(())
    }
}

unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    ::std::slice::from_raw_parts((p as *const T) as *const u8, ::std::mem::size_of::<T>())
}

impl WriteTo for SocketAddr {
    fn write_to<T: Write>(&self, writer: &mut T) -> std::io::Result<()> {
        writer.write_all(unsafe { any_as_u8_slice(&self) })?;
        Ok(())
    }
}

impl<U: WriteTo> WriteTo for Option<U> {
    fn write_to<T: Write>(&self, writer: &mut T) -> std::io::Result<()> {
        writer.write_all(&[self.is_some() as u8])?;
        if let Some(value) = &self {
            value.write_to(writer)?;
        }
        Ok(())
    }
}

impl WriteTo for bool {
    fn write_to<T: Write>(&self, writer: &mut T) -> std::io::Result<()> {
        writer.write_all(&[*self as u8])?;
        Ok(())
    }
}

macro_rules! ImplReadFromPrimative {
    ($NAME:ident) => {
        impl ReadFrom for $NAME {
            fn read_from<T: Read>(reader: &mut T, length: usize) -> io::Result<(Self, usize)> {
                let mut buffer = [0; std::mem::size_of::<$NAME>()];
                reader.read_exact(&mut buffer)?;
                Ok((
                    $NAME::from_be_bytes(buffer),
                    length - std::mem::size_of::<$NAME>(),
                ))
            }
        }
    };
}

// ImplReadFromPrimative!(u8);
ImplReadFromPrimative!(u16);
ImplReadFromPrimative!(u32);
ImplReadFromPrimative!(u64);
ImplReadFromPrimative!(u128);
ImplReadFromPrimative!(i16);
ImplReadFromPrimative!(i32);
ImplReadFromPrimative!(i64);

impl<const N: usize> ReadFrom for [u8; N] {
    fn read_from<T: Read>(reader: &mut T, length: usize) -> io::Result<(Self, usize)> {
        let mut buffer = [0; N];
        reader.read_exact(&mut buffer)?;
        Ok((buffer, length - N))
    }
}

impl ReadFrom for BitVec {
    fn read_from<T: Read>(reader: &mut T, length: usize) -> io::Result<(Self, usize)> {
        let mut buffer = Vec::new();
        buffer.resize(length as _, 0);
        reader.read_exact(&mut buffer)?;
        let result = BitVec::from_bytes(&buffer);
        Ok((result, 0))
    }
}

impl<R: ReadFrom> ReadFrom for std::vec::Vec<R> {
    fn read_from<T: Read>(reader: &mut T, mut length: usize) -> io::Result<(Self, usize)> {
        let mut buffer = Vec::new();
        while length != 0 {
            let (val, remaining) = R::read_from(reader, length)?;
            length = remaining;
            buffer.push(val);
        }
        Ok((buffer, 0))
    }
}

impl<R: ReadFrom> ReadFrom for Option<R> {
    fn read_from<T: Read>(reader: &mut T, length: usize) -> io::Result<(Self, usize)> {
        let mut buffer = [0u8; 1];
        reader.read(&mut buffer)?;
        if buffer[0] == 0 {
            return Ok((None, length - 1));
        }
        let (val, remaining) = R::read_from(reader, length - 1)?;
        Ok((Some(val), remaining))
    }
}

impl ReadFrom for Vec<u8> {
    fn read_from<T: Read>(reader: &mut T, length: usize) -> io::Result<(Self, usize)> {
        let mut buffer = Vec::new();
        buffer.resize(length as _, 0);
        reader.read_exact(&mut buffer)?;
        Ok((buffer, 0))
    }
}

impl ReadFrom for String {
    fn read_from<T: Read>(reader: &mut T, length: usize) -> io::Result<(Self, usize)> {
        let mut buffer = Vec::new();
        buffer.resize(length as _, 0);
        reader.read_exact(&mut buffer)?;
        let result = String::from_utf8(buffer)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "bad utf8"))?;
        Ok((result, 0))
    }
}

impl ReadFrom for SocketAddr {
    fn read_from<T: Read>(reader: &mut T, length: usize) -> io::Result<(Self, usize)> {
        let mut buffer = [0; std::mem::size_of::<SocketAddr>()];
        let size = std::mem::size_of::<SocketAddr>();
        reader.read_exact(&mut buffer)?;
        let result: SocketAddr = unsafe { std::mem::transmute(buffer) };
        Ok((result, length - size))
    }
}

impl ReadFrom for bool {
    fn read_from<T: Read>(reader: &mut T, length: usize) -> io::Result<(Self, usize)> {
        let mut buffer = [0; std::mem::size_of::<bool>()];
        let size = std::mem::size_of::<bool>();
        reader.read_exact(&mut buffer)?;
        let result: bool = unsafe { std::mem::transmute(buffer) };
        Ok((result, length - size))
    }
}

#[derive(WriteTo, ReadFrom, Length, NormalizedIntegerAccessors)]
struct TestInner {
    pub first: i32,
    pub second: u64,
}

// #[derive(WriteTo, ReadFrom, Length, NormalizedIntegerAccessors)]
#[derive(WriteTo, NormalizedIntegerAccessors)]
struct Pancakes {
    item: u32,
    array: [u8; 2],
    test_vec: std::vec::Vec<TestInner>,
}

#[cfg(test)]
mod tests {
    use crate::Pancakes;

    #[test]
    fn it_works() {
        let test = Pancakes {
            item: 5,
            array: [1, 1],
            test_vec: vec![],
        };
        assert_eq!(test.get_item(), 5);
    }
}
