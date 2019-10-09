//! Client which can read and write data from InfluxDB.
//!
//! # Arguments
//!
//!  * `url`: The URL where InfluxDB is running (ex. `http://localhost:8086`).
//!  * `database`: The Database against which queries and writes will be run.
//!
//! # Examples
//!
//! ```rust
//! use influxdb::client::InfluxDbClient;
//!
//! let client = InfluxDbClient::new("http://localhost:8086", "test");
//!
//! assert_eq!(client.database_name(), "test");
//! ```

use futures::{Future, Stream};
use reqwest::r#async::{Client, Decoder};
use reqwest::{StatusCode, Url};

use std::mem;

use crate::error::InfluxDbError;
use crate::query::{InfluxDbQuery, InfluxDbQueryTypes};

#[derive(Clone, Debug)]
/// Internal Authentication representation
pub(crate) struct InfluxDbAuthentication {
    pub username: String,
    pub password: String,
}

#[derive(Clone, Debug)]
/// Internal Representation of a Client
pub struct InfluxDbClient {
    url: String,
    database: String,
    auth: Option<InfluxDbAuthentication>,
}

impl Into<Vec<(String, String)>> for InfluxDbClient {
    fn into(self) -> Vec<(String, String)> {
        let mut vec: Vec<(String, String)> = Vec::new();
        vec.push(("db".to_string(), self.database));
        if let Some(auth) = self.auth {
            vec.push(("u".to_string(), auth.username));
            vec.push(("p".to_string(), auth.password));
        }
        vec
    }
}

impl<'a> Into<Vec<(String, String)>> for &'a InfluxDbClient {
    fn into(self) -> Vec<(String, String)> {
        let mut vec: Vec<(String, String)> = Vec::new();
        vec.push(("db".to_string(), self.database.to_owned()));
        if let Some(auth) = &self.auth {
            vec.push(("u".to_string(), auth.username.to_owned()));
            vec.push(("p".to_string(), auth.password.to_owned()));
        }
        vec
    }
}

impl InfluxDbClient {
    /// Instantiates a new [`InfluxDbClient`](crate::client::InfluxDbClient)
    ///
    /// # Arguments
    ///
    ///  * `url`: The URL where InfluxDB is running (ex. `http://localhost:8086`).
    ///  * `database`: The Database against which queries and writes will be run.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use influxdb::client::InfluxDbClient;
    ///
    /// let _client = InfluxDbClient::new("http://localhost:8086", "test");
    /// ```
    pub fn new<S1, S2>(url: S1, database: S2) -> Self
    where
        S1: ToString,
        S2: ToString,
    {
        InfluxDbClient {
            url: url.to_string(),
            database: database.to_string(),
            auth: None,
        }
    }

    /// Add authentication/authorization information to [`InfluxDbClient`](crate::client::InfluxDbClient)
    ///
    /// # Arguments
    ///
    /// * username: The Username for InfluxDB.
    /// * password: THe Password for the user.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use influxdb::client::InfluxDbClient;
    ///
    /// let _client = InfluxDbClient::new("http://localhost:9086", "test").with_auth("admin", "password");
    /// ```
    pub fn with_auth<S1, S2>(mut self, username: S1, password: S2) -> Self
    where
        S1: ToString,
        S2: ToString,
    {
        self.auth = Some(InfluxDbAuthentication {
            username: username.to_string(),
            password: password.to_string(),
        });
        self
    }

    /// Returns the name of the database the client is using
    pub fn database_name(&self) -> &str {
        &self.database
    }

    /// Returns the URL of the InfluxDB installation the client is using
    pub fn database_url(&self) -> &str {
        &self.url
    }

    /// Pings the InfluxDB Server
    ///
    /// Returns a tuple of build type and version number
    pub fn ping(&self) -> impl Future<Item = (String, String), Error = InfluxDbError> {
        Client::new()
            .get(format!("{}/ping", self.url).as_str())
            .send()
            .map(|res| {
                let build = res
                    .headers()
                    .get("X-Influxdb-Build")
                    .unwrap()
                    .to_str()
                    .unwrap();
                let version = res
                    .headers()
                    .get("X-Influxdb-Version")
                    .unwrap()
                    .to_str()
                    .unwrap();

                (String::from(build), String::from(version))
            })
            .map_err(|err| InfluxDbError::ProtocolError {
                error: format!("{}", err),
            })
    }

    /// Sends a [`InfluxDbReadQuery`](crate::query::read_query::InfluxDbReadQuery) or [`InfluxDbWriteQuery`](crate::query::write_query::InfluxDbWriteQuery) to the InfluxDB Server.
    ///
    /// A version capable of parsing the returned string is available under the [serde_integration](crate::integrations::serde_integration)
    ///
    /// # Arguments
    ///
    ///  * `q`: Query of type [`InfluxDbReadQuery`](crate::query::read_query::InfluxDbReadQuery) or [`InfluxDbWriteQuery`](crate::query::write_query::InfluxDbWriteQuery)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use influxdb::client::InfluxDbClient;
    /// use influxdb::query::{InfluxDbQuery, Timestamp};
    ///
    /// let client = InfluxDbClient::new("http://localhost:8086", "test");
    /// let _future = client.query(
    ///     &InfluxDbQuery::write_query(Timestamp::NOW, "weather")
    ///         .add_field("temperature", 82)
    /// );
    /// ```
    /// # Errors
    ///
    /// If the function can not finish the query,
    /// a [`InfluxDbError`] variant will be returned.
    ///
    /// [`InfluxDbError`]: enum.InfluxDbError.html
    pub fn query<'q, Q>(&self, q: &'q Q) -> Box<dyn Future<Item = String, Error = InfluxDbError>>
    where
        Q: InfluxDbQuery,
        &'q Q: Into<InfluxDbQueryTypes<'q>>,
    {
        use futures::future;

        let query = match q.build() {
            Err(err) => {
                let error = InfluxDbError::InvalidQueryError {
                    error: format!("{}", err),
                };
                return Box::new(future::err::<String, InfluxDbError>(error));
            }
            Ok(query) => query,
        };

        let basic_parameters: Vec<(String, String)> = self.into();

        let client = match q.into() {
            InfluxDbQueryTypes::Read(_) => {
                let read_query = query.get();
                let mut url = match Url::parse_with_params(
                    format!("{url}/query", url = self.database_url()).as_str(),
                    basic_parameters,
                ) {
                    Ok(url) => url,
                    Err(err) => {
                        let error = InfluxDbError::UrlConstructionError {
                            error: format!("{}", err),
                        };
                        return Box::new(future::err::<String, InfluxDbError>(error));
                    }
                };
                url.query_pairs_mut().append_pair("q", &read_query.clone());

                if read_query.contains("SELECT") || read_query.contains("SHOW") {
                    Client::new().get(url)
                } else {
                    Client::new().post(url)
                }
            }
            InfluxDbQueryTypes::Write(write_query) => {
                let mut url = match Url::parse_with_params(
                    format!("{url}/write", url = self.database_url()).as_str(),
                    basic_parameters,
                ) {
                    Ok(url) => url,
                    Err(err) => {
                        let error = InfluxDbError::InvalidQueryError {
                            error: format!("{}", err),
                        };
                        return Box::new(future::err::<String, InfluxDbError>(error));
                    }
                };
                url.query_pairs_mut()
                    .append_pair("precision", &write_query.get_precision());
                Client::new().post(url).body(query.get())
            }
        };
        Box::new(
            client
                .send()
                .map_err(|err| InfluxDbError::ConnectionError { error: err })
                .and_then(
                    |res| -> future::FutureResult<reqwest::r#async::Response, InfluxDbError> {
                        match res.status() {
                            StatusCode::UNAUTHORIZED => {
                                futures::future::err(InfluxDbError::AuthorizationError)
                            }
                            StatusCode::FORBIDDEN => {
                                futures::future::err(InfluxDbError::AuthenticationError)
                            }
                            _ => futures::future::ok(res),
                        }
                    },
                )
                .and_then(|mut res| {
                    let body = mem::replace(res.body_mut(), Decoder::empty());
                    body.concat2().map_err(|err| InfluxDbError::ProtocolError {
                        error: format!("{}", err),
                    })
                })
                .and_then(|body| {
                    if let Ok(utf8) = std::str::from_utf8(&body) {
                        let s = utf8.to_owned();

                        // todo: improve error parsing without serde
                        if s.contains("\"error\"") {
                            return futures::future::err(InfluxDbError::DatabaseError {
                                error: format!("influxdb error: \"{}\"", s),
                            });
                        }

                        return futures::future::ok(s);
                    }

                    futures::future::err(InfluxDbError::DeserializationError {
                        error: "response could not be converted to UTF-8".to_string(),
                    })
                }),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::client::InfluxDbClient;

    #[test]
    fn test_fn_database() {
        let client = InfluxDbClient::new("http://localhost:8068", "database");
        assert_eq!("database", client.database_name());
    }

    #[test]
    fn test_with_auth() {
        let client = InfluxDbClient::new("http://localhost:8068", "database");
        assert_eq!(client.url, "http://localhost:8068");
        assert_eq!(client.database, "database");
        assert!(client.auth.is_none());
        let with_auth = client.with_auth("username", "password");
        assert!(with_auth.auth.is_some());
        let auth = with_auth.auth.unwrap();
        assert_eq!(&auth.username, "username");
        assert_eq!(&auth.password, "password");
    }

    #[test]
    fn test_into_impl() {
        let client = InfluxDbClient::new("http://localhost:8068", "database");
        assert!(client.auth.is_none());
        let basic_parameters: Vec<(String, String)> = client.into();
        assert_eq!(
            vec![("db".to_string(), "database".to_string())],
            basic_parameters
        );

        let with_auth = InfluxDbClient::new("http://localhost:8068", "database")
            .with_auth("username", "password");
        let basic_parameters_with_auth: Vec<(String, String)> = with_auth.into();
        assert_eq!(
            vec![
                ("db".to_string(), "database".to_string()),
                ("u".to_string(), "username".to_string()),
                ("p".to_string(), "password".to_string())
            ],
            basic_parameters_with_auth
        );

        let client = InfluxDbClient::new("http://localhost:8068", "database");
        assert!(client.auth.is_none());
        let basic_parameters: Vec<(String, String)> = (&client).into();
        assert_eq!(
            vec![("db".to_string(), "database".to_string())],
            basic_parameters
        );

        let with_auth = InfluxDbClient::new("http://localhost:8068", "database")
            .with_auth("username", "password");
        let basic_parameters_with_auth: Vec<(String, String)> = (&with_auth).into();
        assert_eq!(
            vec![
                ("db".to_string(), "database".to_string()),
                ("u".to_string(), "username".to_string()),
                ("p".to_string(), "password".to_string())
            ],
            basic_parameters_with_auth
        );
    }
}
