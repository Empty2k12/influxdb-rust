<div align="center">
    <br/>
    <img
        alt="rust-influxdb"
        src="https://i.imgur.com/4k7l8XJ.png"
        width=250px />
    <br/>
    <br/>
    <strong>Unofficial InfluxDB Driver for Rust</strong>
</div>
<br/>
<p align="center">
    <a href="https://crates.io/crates/influxdb">
        <img src="https://img.shields.io/crates/v/influxdb.svg"/>
    </a>
    <a href="https://travis-ci.org/Empty2k12/influxdb-rust">
        <img src="https://travis-ci.org/Empty2k12/influxdb-rust.svg?branch=master" alt='Build Status' />
    </a>
    <a href="https://docs.rs/crate/influxdb">
        <img src="https://docs.rs/influxdb/badge.svg" alt='Documentation Status' />
    </a>
    <a href="https://www.rust-lang.org/en-US/">
        <img src="https://img.shields.io/badge/Made%20with-Rust-orange.svg" alt='Build with Rust' />
    </a>
</p>

This library is a work in progress. Although we've been using it in production at [OpenVelo](https://openvelo.org/),
we've prioritized features that fit our use cases. This means a feature you might need is not implemented
yet or could be handled better.

Pull requests are always welcome. See [Contributing](https://github.com/Empty2k12/influxdb-rust/blob/master/CONTRIBUTING.md) and [Code of Conduct](https://github.com/Empty2k12/influxdb-rust/blob/master/CODE_OF_CONDUCT.md).

### Currently Supported Features

-   Reading and Writing to InfluxDB
-   Optional Serde Support for Deserialization
-   Running multiple queries in one request (e.g. `SELECT * FROM weather_berlin; SELECT * FROM weather_london`)
-   Authenticated and Unauthenticated Connections
-   Optional conversion between `Timestamp` and `Chrono::DateTime<Utc>` via `chrono_timestamps` compilation feature
### Planned Features

-   Read Query Builder instead of supplying raw queries
-   `#[derive(InfluxDbReadable)]` and `#[derive(InfluxDbWriteable)]` proc macros

## Quickstart

Add the following to your `Cargo.toml`

```toml
influxdb = "0.0.5"
```

For an example with using Serde deserialization, please refer to [serde_integration](crate::integrations::serde_integration)

```rust
use influxdb::{Client, Query, Timestamp};
use serde::Deserialize;
use tokio::runtime::current_thread::Runtime;

// Create a Client with URL `http://localhost:8086`
// and database name `test`
let client = Client::new("http://localhost:8086", "test");

// Let's write something to InfluxDB. First we're creating a
// WriteQuery to write some data.
// This creates a query which writes a new measurement into a series called `weather`
let write_query = Query::write_query(Timestamp::Now, "weather")
    .add_field("temperature", 82);

// Since this library is async by default, we're going to need a Runtime,
// which can asynchonously run our query.
// The [tokio](https://crates.io/crates/tokio) crate lets us easily create a new Runtime.
let mut rt = Runtime::new().expect("Unable to create a runtime");

// To actually submit the data to InfluxDB, the `block_on` method can be used to
// halt execution of our program until it has been completed.
let write_result = rt.block_on(client.query(&write_query));
assert!(write_result.is_ok(), "Write result was not okay");

// Reading data is as simple as writing. First we need to create a query
let read_query = Query::raw_read_query("SELECT * FROM weather");

// Again, we're blocking until the request is done
let read_result = rt.block_on(client.query(&read_query));

assert!(read_result.is_ok(), "Read result was not ok");

// We can be sure the result was successful, so we can unwrap the result to get
// the response String from InfluxDB
println!("{}", read_result.unwrap());
```

For further examples, check out the Integration Tests in `tests/integration_tests.rs`
in the repository.

## License

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

@ 2019 Gero Gerke, All rights reserved.
