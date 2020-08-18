extern crate async_graphql;
extern crate async_graphql_derive;

use crate::{Version, VersionReq};
use async_graphql::{InputValueError, InputValueResult, ScalarType, Value};
use async_graphql_derive::Scalar;

/// `Version` Semantic Versioning https://semver.org
#[Scalar(name = "Version")]
impl ScalarType for Version {
    fn parse(value: Value) -> InputValueResult<Self> {
        match value {
            Value::String(v) => match Version::parse(v.as_ref()) {
                Ok(v) => Ok(v),
                Err(e) => Err(InputValueError::Custom(format!("{}", e))),
            },
            _ => Err(InputValueError::ExpectedType(value)),
        }
    }

    fn to_value(&self) -> Value {
        Value::String(self.to_string())
    }
}

/// `VersionReq` Semantic Version requirement https://semver.org
#[Scalar(name = "VersionReq")]
impl ScalarType for VersionReq {
    fn parse(value: Value) -> InputValueResult<Self> {
        match value {
            Value::String(v) => match VersionReq::parse(v.as_ref()) {
                Ok(v) => Ok(v),
                Err(e) => Err(InputValueError::Custom(format!("{}", e))),
            },
            _ => Err(InputValueError::ExpectedType(value)),
        }
    }

    fn to_value(&self) -> Value {
        Value::String(self.to_string())
    }
}
