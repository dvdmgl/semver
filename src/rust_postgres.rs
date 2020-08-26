use byteorder::{BigEndian, ReadBytesExt};
use bytes::{BufMut, BytesMut};
use postgres_types::{to_sql_checked, FromSql, IsNull, ToSql, Type};

use std::error::Error;

use crate::{
    version_req::{Op, WildcardVersion},
    Version, VersionReq,
};

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

impl<'a> VersionReq {
    /// creates a query with upper and lower range, returning results should be
    /// filtered by `matches`
    pub fn gen_query(
        &self,
        field: &str,
        table: &str,
        start: u32,
        sort: bool,
    ) -> (String, Vec<Version>, u32) {
        let mut out = Vec::with_capacity(self.predicates.len());
        let mut versions = Vec::new();
        let mut i = start;
        for pred in &self.predicates {
            i += 1;
            // get query params
            out.push(match pred.op {
                Op::Ex => format!("\"{}\".\"{}\" = ${}", table, field, i),
                Op::Gt => format!("{} > ${}", field, i),
                Op::GtEq => format!("{} >= ${}", field, i),
                Op::Lt => format!("{} < ${}", field, i),
                Op::LtEq => format!("{} <= ${}", field, i),
                _ => {
                    i += 1;
                    format!(
                        "(\"{}\".\"{}\" >= ${} AND \"{}\".\"{}\" < ${})",
                        table,
                        field,
                        i - 1,
                        table,
                        field,
                        i
                    )
                }
            });
            let (major, minor, patch, pre) =
                (pred.major, pred.minor, pred.patch, &pred.pre);
            // get min version
            match pred.op {
                Op::Ex
                | Op::Gt
                | Op::GtEq
                | Op::Lt
                | Op::LtEq
                | Op::Tilde
                | Op::Compatible => {
                    versions.push(Version {
                        major,
                        minor: minor.unwrap_or_default(),
                        patch: patch.unwrap_or_default(),
                        pre: pre.to_vec(),
                        build: vec![],
                    });
                }
                Op::Wildcard(ref v) => match &v {
                    WildcardVersion::Patch => {
                        versions.push(Version {
                            major,
                            minor: minor.unwrap_or_default(),
                            patch: 0,
                            pre: vec![],
                            build: vec![],
                        });
                    }
                    WildcardVersion::Minor => {
                        versions.push(Version {
                            major,
                            minor: 0,
                            patch: 0,
                            pre: vec![],
                            build: vec![],
                        });
                    }
                    _ => {}
                },
            }
            // get max version
            match pred.op {
                Op::Tilde => versions.push(match (minor, patch) {
                    (Some(minor), Some(_)) => Version {
                        major,
                        minor: minor + 1,
                        patch: 0,
                        pre: pre.to_vec(),
                        build: vec![],
                    },
                    (Some(minor), None) => Version {
                        major,
                        minor: minor + 1,
                        patch: 0,
                        pre: vec![],
                        build: vec![],
                    },
                    _ => Version {
                        major: major + 1,
                        minor: 0,
                        patch: 0,
                        pre: vec![],
                        build: vec![],
                    },
                }),
                Op::Compatible => versions.push(match (major, minor, patch) {
                    (major, Some(minor), Some(patch))
                        if major == 0 && minor == 0 =>
                    {
                        // incr patch and let it check for meta
                        Version {
                            major,
                            minor,
                            patch: patch + 1,
                            pre: vec![],
                            build: vec![],
                        }
                    }
                    (major, Some(minor), _) if major == 0 => Version {
                        major,
                        minor: minor + 1,
                        patch: 0,
                        pre: vec![],
                        build: vec![],
                    },
                    (major, _, _) => Version {
                        major: major + 1,
                        minor: 0,
                        patch: 0,
                        pre: vec![],
                        build: vec![],
                    },
                }),
                Op::Wildcard(ref v) => match v {
                    WildcardVersion::Patch => {
                        versions.push(Version {
                            major,
                            minor: minor.unwrap_or_default() + 1,
                            patch: 0,
                            pre: vec![],
                            build: vec![],
                        });
                    }
                    WildcardVersion::Minor => {
                        versions.push(Version {
                            major: major + 1,
                            minor: 0,
                            patch: 0,
                            pre: vec![],
                            build: vec![],
                        });
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        let mut qry = out.join(" OR ");
        if sort {
            qry.push_str(
                format!(" ORDER BY \"{}\".\"{}\" ASC", table, field).as_ref(),
            );
        };
        (qry, versions, i)
    }
}
