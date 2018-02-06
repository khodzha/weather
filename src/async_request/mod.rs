extern crate futures;
extern crate hyper;
extern crate tokio_core;
extern crate serde_json;

use std;
use std::io;
use std::time::Duration;

use self::hyper::{Client};
use self::serde_json::Value;
use self::futures::{Future, Stream};
use self::futures::future::Either;
use self::tokio_core::reactor::{Timeout, Handle};

static TIMEOUT: u64 = 10;

pub mod error;

#[derive(Debug)]
pub struct AsyncResponse {
    pub status: hyper::StatusCode,
    pub body: hyper::Chunk
}

#[derive(Debug, Clone)]
pub struct AsyncJsonResponse {
    pub status: hyper::StatusCode,
    pub body: Option<Value>
}

pub type Result<T> = std::result::Result<T, self::error::ApiError>;

pub fn async_request(handle: &Handle, url: &str) -> Box<Future<Item = AsyncResponse, Error = hyper::Error>> {
    let timeout = Timeout::new(Duration::from_secs(TIMEOUT), &handle).unwrap();
    let client = Client::configure().build(&handle);
    let uri = url.parse().unwrap();

    let resp = client.get(uri).and_then(|web_res| {
        let status = web_res.status();
        web_res.body().concat2().map(move |body|
            AsyncResponse {
                status: status,
                body: body
            }
        )
    }).select2(timeout).then(|res| match res {
        Ok(Either::A((got, _timeout))) => Ok(got),
        Ok(Either::B((_timeout_error, _get))) => {
            Err(hyper::Error::Io(io::Error::new(
                io::ErrorKind::TimedOut,
                "timed out while connecting to owm",
            )))
        }
        Err(Either::A((get_error, _timeout))) => Err(get_error),
        Err(Either::B((timeout_error, _get))) => Err(From::from(timeout_error)),
    });

    Box::new(resp)
}

pub fn async_json_request(handle: &Handle, url: &str) -> Box<Future<Item = AsyncJsonResponse, Error = hyper::Error>> {
    let resp = async_request(handle, url).and_then(|async_resp| {
        let v: serde_json::Result<Value> = serde_json::from_slice(&async_resp.body);
        Ok(match v {
            Ok(parsed_body) => AsyncJsonResponse { status: async_resp.status, body: Some(parsed_body) },
            Err(_) => AsyncJsonResponse { status: async_resp.status, body: None },
        })
    });
    Box::new(resp)
}

#[cfg(test)]
extern crate mockito;

#[cfg(test)]
const URL: &'static str = mockito::SERVER_URL;

#[cfg(test)]
mod tests {
    use super::*;
    use self::mockito::{mock};

    #[test]
    fn it_performs_request() {
        let m = mock("GET", "/some-url")
            .with_status(200)
            .with_body(r#""string-json-response""#)
            .create();

        let mut core = tokio_core::reactor::Core::new().unwrap();
        let handle = core.handle();
        let work = async_json_request(&handle, &format!("{}/some-url", URL));
        let r = core.run(work);

        assert_eq!(r.unwrap().body.unwrap(), "string-json-response");
        m.assert();
    }

    #[test]
    fn it_handles_unparsable_json() {
        let m = mock("GET", "/some-url")
            .with_status(200)
            .with_body(r#""#)
            .create();

        let mut core = tokio_core::reactor::Core::new().unwrap();
        let handle = core.handle();
        let work = async_json_request(&handle, &format!("{}/some-url", URL));
        let r = core.run(work);

        assert!(r.unwrap().body.is_none());
        m.assert();
    }
}
