//! Used to create queries of type [`ReadQuery`](crate::query::read_query::ReadQuery) or
//! [`WriteQuery`](crate::query::write_query::WriteQuery) which can be executed in InfluxDB
//!
//! # Examples
//!
//! ```rust
//! use influxdb::{Query, Timestamp};
//!
//! let write_query = Query::write_query(Timestamp::Now, "measurement")
//!     .add_field("field1", 5)
//!     .add_tag("author", "Gero")
//!     .build();
//!
//! assert!(write_query.is_ok());
//!
//! let read_query = Query::raw_read_query("SELECT * FROM weather")
//!     .build();
//!
//! assert!(read_query.is_ok());
//! ```

#[cfg(any(test, feature = "chrono_timestamps"))]
extern crate chrono;
#[cfg(any(test, feature = "chrono_timestamps"))]
use chrono::prelude::{DateTime, TimeZone, Utc};
#[cfg(any(test, feature = "chrono_timestamps"))]
use std::convert::TryInto;

pub mod read_query;
pub mod write_query;

use std::fmt;

use crate::{Error, ReadQuery, WriteQuery};

#[derive(PartialEq, Debug)]
pub enum Timestamp {
    Now,
    Nanoseconds(usize),
    Microseconds(usize),
    Milliseconds(usize),
    Seconds(usize),
    Minutes(usize),
    Hours(usize),
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Timestamp::*;
        match self {
            Now => write!(f, ""),
            Nanoseconds(ts) | Microseconds(ts) | Milliseconds(ts) | Seconds(ts) | Minutes(ts)
            | Hours(ts) => write!(f, "{}", ts),
        }
    }
}

#[cfg(any(test, feature = "chrono_timestamps"))]
impl Into<DateTime<Utc>> for Timestamp {
    fn into(self) -> DateTime<Utc> {
        match self {
            Timestamp::Now => Utc::now(),
            Timestamp::Hours(h) => {
                let millis = h * 60 * 60 * 1000;
                Utc.timestamp_millis(millis.try_into().unwrap())
            }
            Timestamp::Minutes(m) => {
                let millis = m * 60 * 1000;
                Utc.timestamp_millis(millis.try_into().unwrap())
            }
            Timestamp::Seconds(m) => {
                let millis = m * 1000;
                Utc.timestamp_millis(millis.try_into().unwrap())
            }
            Timestamp::Milliseconds(millis) => Utc.timestamp_millis(millis.try_into().unwrap()),
            Timestamp::Nanoseconds(ns) => Utc.timestamp_nanos(ns.try_into().unwrap()),
            Timestamp::Microseconds(mis) => {
                let nanos = mis / 10000;
                Utc.timestamp_nanos(nanos.try_into().unwrap())
            }
        }
    }
}

#[cfg(any(test, feature = "chrono_timestamps"))]
impl<T> From<DateTime<T>> for Timestamp
where
    T: TimeZone,
{
    fn from(date_time: DateTime<T>) -> Self {
        Timestamp::Milliseconds(date_time.timestamp_millis() as usize)
    }
}

/// Internal enum used to represent either type of query.
pub enum QueryTypes<'a> {
    Read(&'a ReadQuery),
    Write(&'a WriteQuery),
}

impl<'a> From<&'a ReadQuery> for QueryTypes<'a> {
    fn from(query: &'a ReadQuery) -> Self {
        Self::Read(query)
    }
}

impl<'a> From<&'a WriteQuery> for QueryTypes<'a> {
    fn from(query: &'a WriteQuery) -> Self {
        Self::Write(query)
    }
}

pub trait Query {
    /// Builds valid InfluxSQL which can be run against the Database.
    /// In case no fields have been specified, it will return an error,
    /// as that is invalid InfluxSQL syntax.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use influxdb::{Query, Timestamp};
    ///
    /// let invalid_query = Query::write_query(Timestamp::Now, "measurement").build();
    /// assert!(invalid_query.is_err());
    ///
    /// let valid_query = Query::write_query(Timestamp::Now, "measurement").add_field("myfield1", 11).build();
    /// assert!(valid_query.is_ok());
    /// ```
    fn build(&self) -> Result<ValidQuery, Error>;

    fn get_type(&self) -> QueryType;
}

impl dyn Query {
    /// Returns a [`WriteQuery`](crate::WriteQuery) builder.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use influxdb::{Query, Timestamp};
    ///
    /// Query::write_query(Timestamp::Now, "measurement"); // Is of type [`WriteQuery`](crate::WriteQuery)
    /// ```
    pub fn write_query<S>(timestamp: Timestamp, measurement: S) -> WriteQuery
    where
        S: Into<String>,
    {
        WriteQuery::new(timestamp, measurement)
    }

    /// Returns a [`ReadQuery`](crate::ReadQuery) builder.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use influxdb::Query;
    ///
    /// Query::raw_read_query("SELECT * FROM weather"); // Is of type [`ReadQuery`](crate::ReadQuery)
    /// ```
    pub fn raw_read_query<S>(read_query: S) -> ReadQuery
    where
        S: Into<String>,
    {
        ReadQuery::new(read_query)
    }
}

#[derive(Debug)]
#[doc(hidden)]
pub struct ValidQuery(String);
impl ValidQuery {
    pub fn get(self) -> String {
        self.0
    }
}
impl<T> From<T> for ValidQuery
where
    T: Into<String>,
{
    fn from(string: T) -> Self {
        Self(string.into())
    }
}
impl PartialEq<String> for ValidQuery {
    fn eq(&self, other: &String) -> bool {
        &self.0 == other
    }
}
impl PartialEq<&str> for ValidQuery {
    fn eq(&self, other: &&str) -> bool {
        &self.0 == other
    }
}

/// Internal Enum used to decide if a `POST` or `GET` request should be sent to InfluxDB. See [InfluxDB Docs](https://docs.influxdata.com/influxdb/v1.7/tools/api/#query-http-endpoint).
#[derive(PartialEq, Debug)]
pub enum QueryType {
    ReadQuery,
    WriteQuery,
}

#[cfg(test)]
mod tests {
    extern crate chrono;
    use crate::query::{Timestamp, ValidQuery};
    use chrono::prelude::{DateTime, TimeZone, Utc};

    const MINUTES_PER_HOUR: i64 = 60;
    const SECONDS_PER_MINUTE: i64 = 60;
    const MILLIS_PER_SECOND: i64 = 1000;
    const MICROS_PER_NANO: i64 = 1000;

    #[test]
    fn test_equality_str() {
        assert_eq!(ValidQuery::from("hello"), "hello");
    }

    #[test]
    fn test_equality_string() {
        assert_eq!(
            ValidQuery::from(String::from("hello")),
            String::from("hello")
        );
    }

    #[test]
    fn test_format_for_timestamp_now() {
        assert!(format!("{}", Timestamp::Now) == "");
    }

    #[test]
    fn test_format_for_timestamp_else() {
        assert!(format!("{}", Timestamp::Nanoseconds(100)) == "100");
    }

    #[test]
    fn test_chrono_datetime_from_timestamp_now() {
        let datetime_from_timestamp: DateTime<Utc> = Timestamp::Now.into();
        assert_eq!(Utc::now().date(), datetime_from_timestamp.date())
    }
    #[test]
    fn test_chrono_datetime_from_timestamp_hours() {
        let datetime_from_timestamp: DateTime<Utc> = Timestamp::Hours(2).into();
        assert_eq!(
            Utc.timestamp_millis(2 * MINUTES_PER_HOUR * SECONDS_PER_MINUTE * MILLIS_PER_SECOND),
            datetime_from_timestamp
        )
    }
    #[test]
    fn test_chrono_datetime_from_timestamp_minutes() {
        let datetime_from_timestamp: DateTime<Utc> = Timestamp::Minutes(2).into();
        assert_eq!(
            Utc.timestamp_millis(2 * SECONDS_PER_MINUTE * MILLIS_PER_SECOND),
            datetime_from_timestamp
        )
    }
    #[test]
    fn test_chrono_datetime_from_timestamp_seconds() {
        let datetime_from_timestamp: DateTime<Utc> = Timestamp::Seconds(2).into();
        assert_eq!(
            Utc.timestamp_millis(2 * MILLIS_PER_SECOND),
            datetime_from_timestamp
        )
    }
    #[test]
    fn test_chrono_datetime_from_timestamp_millis() {
        let datetime_from_timestamp: DateTime<Utc> = Timestamp::Milliseconds(2).into();
        assert_eq!(Utc.timestamp_millis(2), datetime_from_timestamp)
    }

    #[test]
    fn test_chrono_datetime_from_timestamp_nanos() {
        let datetime_from_timestamp: DateTime<Utc> = Timestamp::Nanoseconds(1).into();
        assert_eq!(Utc.timestamp_nanos(1), datetime_from_timestamp)
    }

    #[test]
    fn test_chrono_datetime_from_timestamp_micros() {
        let datetime_from_timestamp: DateTime<Utc> = Timestamp::Microseconds(1).into();
        assert_eq!(
            Utc.timestamp_nanos(1 / MICROS_PER_NANO),
            datetime_from_timestamp
        )
    }

    #[test]
    fn test_timestamp_from_chrono_date() {
        let timestamp_from_datetime: Timestamp = Utc.ymd(1970, 1, 1).and_hms(0, 0, 1).into();
        assert_eq!(Timestamp::Milliseconds(1000), timestamp_from_datetime)
    }
}
