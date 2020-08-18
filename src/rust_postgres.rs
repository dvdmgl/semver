use byteorder::{BigEndian, ReadBytesExt};
use bytes::{BufMut, BytesMut};
use postgres_types::{to_sql_checked, FromSql, IsNull, ToSql, Type};

use std::error::Error;

use crate::Version;

macro_rules! accepts_semver {
    () => {
        fn accepts(ty: &Type) -> bool {
            match ty.name() {
                "semver64" => true,
                _ => false,
            }
        }
    };
}

/// Rust postgres with semver64 extention
impl<'a> FromSql<'a> for Version {
    #[inline]
    fn from_sql(
        _: &Type,
        mut buf: &[u8],
    ) -> Result<Self, Box<dyn Error + Sync + Send>> {
        let major = buf.read_u64::<BigEndian>()?;
        let minor = buf.read_u64::<BigEndian>()?;
        let patch = buf.read_u64::<BigEndian>()?;
        match std::str::from_utf8(buf) {
            Ok(s) if !s.is_empty() => {
                // add pre-release '-' if does not start with '+'
                let tmp = if s.starts_with('+') {
                    format!("{}.{}.{}{}", major, minor, patch, s)
                } else {
                    format!("{}.{}.{}-{}", major, minor, patch, s)
                };
                Ok(Self::parse(&tmp)?)
            }
            _ => Ok(Self {
                major,
                minor,
                patch,
                pre: Vec::new(),
                build: Vec::new(),
            }),
        }
    }

    accepts_semver!();
}

/// Rust postgres with semver64 extention
impl ToSql for Version {
    #[inline]
    fn to_sql(
        &self,
        _: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        let mut meta = String::new();
        if !self.pre.is_empty() {
            // don't include the '-'
            for (i, x) in self.pre.iter().enumerate() {
                if i != 0 {
                    meta.push('.');
                }
                meta.push_str(format!("{}", x).as_ref());
            }
        }
        if !self.build.is_empty() {
            meta.push('+');
            for (i, x) in self.build.iter().enumerate() {
                if i != 0 {
                    meta.push('.');
                }
                meta.push_str(format!("{}", x).as_ref());
            }
        }
        out.put_u64(self.major);
        out.put_u64(self.minor);
        out.put_u64(self.patch);
        out.put_slice(meta.as_bytes());
        Ok(IsNull::No)
    }
    to_sql_checked!();
    accepts_semver!();
}
