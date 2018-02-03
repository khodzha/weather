extern crate futures;
extern crate hyper;
extern crate tokio_core;
extern crate serde_json;

use std::io;
use std::time::Duration;

use self::hyper::{Client};
use self::serde_json::Value;
use self::futures::{Future, Stream};
use self::futures::future::{Either};
use self::tokio_core::reactor::{Timeout, Handle};

static TIMEOUT: u64 = 5;

pub fn async_request(handle: &Handle, url: String) -> Box<Future<Item = Value, Error = hyper::Error>> {
    let timeout = Timeout::new(Duration::from_secs(TIMEOUT), &handle).unwrap();
    let client = Client::configure().build(&handle);
    let uri = url.parse().unwrap();

    let resp = client.get(uri).and_then(|web_res| {
        web_res.body().concat2()
    }).and_then(|body| {
        let v: Value = serde_json::from_slice(&body).map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                e
            )
        })?;
        Ok(v)
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
