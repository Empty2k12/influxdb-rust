//! Write Query Builder returned by Query::write_query
//!
//! Can only be instantiated by using Query::write_query

use crate::query::{QueryType, ValidQuery};
use crate::{Error, Query, Timestamp};
use std::fmt::{Display, Formatter};

// todo: batch write queries

pub trait WriteField {
    fn add_to_fields(self, tag: String, fields: &mut Vec<(String, String)>);
}

impl<T: Into<Type>> WriteField for T {
    fn add_to_fields(self, tag: String, fields: &mut Vec<(String, String)>) {
        let val: Type = self.into();
        fields.push((tag, val.to_string()));
    }
}

impl<T: Into<Type>> WriteField for Option<T> {
    fn add_to_fields(self, tag: String, fields: &mut Vec<(String, String)>) {
        if let Some(val) = self {
            val.add_to_fields(tag, fields);
        }
    }
}

/// Internal Representation of a Write query that has not yet been built
pub struct WriteQuery {
    fields: Vec<(String, String)>,
    tags: Vec<(String, String)>,
    measurement: String,
    timestamp: Timestamp,
}

impl WriteQuery {
    /// Creates a new [`WriteQuery`](crate::query::write_query::WriteQuery)
    pub fn new<S>(timestamp: Timestamp, measurement: S) -> Self
    where
        S: Into<String>,
    {
        WriteQuery {
            fields: vec![],
            tags: vec![],
            measurement: measurement.into(),
            timestamp,
        }
    }

    /// Adds a field to the [`WriteQuery`](crate::WriteQuery)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use influxdb::{Query, Timestamp};
    ///
    /// Query::write_query(Timestamp::Now, "measurement").add_field("field1", 5).build();
    /// ```
    pub fn add_field<S, F>(mut self, tag: S, value: F) -> Self
    where
        S: Into<String>,
        F: WriteField,
    {
        value.add_to_fields(tag.into(), &mut self.fields);
        self
    }

    /// Adds a tag to the [`WriteQuery`](crate::WriteQuery)
    ///
    /// Please note that a [`WriteQuery`](crate::WriteQuery) requires at least one field. Composing a query with
    /// only tags will result in a failure building the query.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use influxdb::{Query, Timestamp};
    ///
    /// Query::write_query(Timestamp::Now, "measurement")
    ///     .add_tag("field1", 5); // calling `.build()` now would result in a `Err(Error::InvalidQueryError)`
    /// ```
    pub fn add_tag<S, I>(mut self, tag: S, value: I) -> Self
    where
        S: Into<String>,
        I: Into<Type>,
    {
        let val: Type = value.into();
        self.tags.push((tag.into(), val.to_string()));
        self
    }

    pub fn get_precision(&self) -> String {
        let modifier = match self.timestamp {
            Timestamp::Now => return String::from(""),
            Timestamp::Nanoseconds(_) => "ns",
            Timestamp::Microseconds(_) => "u",
            Timestamp::Milliseconds(_) => "ms",
            Timestamp::Seconds(_) => "s",
            Timestamp::Minutes(_) => "m",
            Timestamp::Hours(_) => "h",
        };
        modifier.to_string()
    }
}

pub enum Type {
    Boolean(bool),
    Float(f64),
    SignedInteger(i64),
    UnsignedInteger(u64),
    Text(String),
}

impl Display for Type {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use Type::*;

        match self {
            Boolean(x) => write!(f, "{}", x),
            Float(x) => write!(f, "{}", x),
            SignedInteger(x) => write!(f, "{}", x),
            UnsignedInteger(x) => write!(f, "{}", x),
            Text(text) => write!(f, "\"{text}\"", text = text),
        }
    }
}

macro_rules! from_impl {
        ( $variant:ident => $( $typ:ident ),+ ) => (
                $(
                    impl From<$typ> for Type {
                        fn from(b: $typ) -> Self {
                            Type::$variant(b.into())
                        }
                    }
                )+
        )
}
from_impl! {Boolean => bool}
from_impl! {Float => f32, f64}
from_impl! {SignedInteger => i8, i16, i32, i64}
from_impl! {UnsignedInteger => u8, u16, u32, u64}
from_impl! {Text => String}
impl From<&str> for Type {
    fn from(b: &str) -> Self {
        Type::Text(b.into())
    }
}

impl Query for WriteQuery {
    fn build(&self) -> Result<ValidQuery, Error> {
        if self.fields.is_empty() {
            return Err(Error::InvalidQueryError {
                error: "fields cannot be empty".to_string(),
            });
        }

        let mut tags = self
            .tags
            .iter()
            .map(|(tag, value)| format!("{tag}={value}", tag = tag, value = value))
            .collect::<Vec<String>>()
            .join(",");
        if !tags.is_empty() {
            tags.insert_str(0, ",");
        }
        let fields = self
            .fields
            .iter()
            .map(|(field, value)| format!("{field}={value}", field = field, value = value))
            .collect::<Vec<String>>()
            .join(",");

        Ok(ValidQuery(format!(
            "{measurement}{tags} {fields}{time}",
            measurement = self.measurement,
            tags = tags,
            fields = fields,
            time = match self.timestamp {
                Timestamp::Now => String::from(""),
                _ => format!(" {}", self.timestamp),
            }
        )))
    }

    fn get_type(&self) -> QueryType {
        QueryType::WriteQuery
    }
}

#[cfg(test)]
mod tests {
    use crate::query::{Query, Timestamp};

    #[test]
    fn test_write_builder_empty_query() {
        let query = Query::write_query(Timestamp::Hours(5), "marina_3").build();

        assert!(query.is_err(), "Query was not empty");
    }

    #[test]
    fn test_write_builder_single_field() {
        let query = Query::write_query(Timestamp::Hours(11), "weather")
            .add_field("temperature", 82)
            .build();

        assert!(query.is_ok(), "Query was empty");
        assert_eq!(query.unwrap(), "weather temperature=82 11");
    }

    #[test]
    fn test_write_builder_multiple_fields() {
        let query = Query::write_query(Timestamp::Hours(11), "weather")
            .add_field("temperature", 82)
            .add_field("wind_strength", 3.7)
            .build();

        assert!(query.is_ok(), "Query was empty");
        assert_eq!(
            query.unwrap(),
            "weather temperature=82,wind_strength=3.7 11"
        );
    }

    #[test]
    fn test_write_builder_optional_fields() {
        let query = Query::write_query(Timestamp::Hours(11), "weather")
            .add_field("temperature", Some(82u64))
            .add_field("wind_strength", <Option<u64>>::None)
            .build();

        assert!(query.is_ok(), "Query was empty");
        assert_eq!(query.unwrap(), "weather temperature=82 11");
    }

    #[test]
    fn test_write_builder_only_tags() {
        let query = Query::write_query(Timestamp::Hours(11), "weather")
            .add_tag("season", "summer")
            .build();

        assert!(query.is_err(), "Query missing one or more fields");
    }

    #[test]
    fn test_write_builder_full_query() {
        let query = Query::write_query(Timestamp::Hours(11), "weather")
            .add_field("temperature", 82)
            .add_tag("location", "us-midwest")
            .add_tag("season", "summer")
            .build();

        assert!(query.is_ok(), "Query was empty");
        assert_eq!(
            query.unwrap(),
            "weather,location=\"us-midwest\",season=\"summer\" temperature=82 11"
        );
    }

    #[test]
    fn test_correct_query_type() {
        use crate::query::QueryType;

        let query = Query::write_query(Timestamp::Hours(11), "weather")
            .add_field("temperature", 82)
            .add_tag("location", "us-midwest")
            .add_tag("season", "summer");

        assert_eq!(query.get_type(), QueryType::WriteQuery);
    }
}
