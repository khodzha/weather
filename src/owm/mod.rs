// openweathermap.org api

extern crate futures;
extern crate hyper;
extern crate tokio_core;
extern crate serde_json;

use self::serde_json::Value;
use std::io;
use std::time::Duration;
use self::futures::{Future, Stream};
use self::hyper::{Client};
use self::futures::future::{Either};

use self::tokio_core::reactor::{Timeout, Handle};
static TIMEOUT: u64 = 5;

pub fn current(handle: &Handle, q: String) -> Box<Future<Item = Value, Error = hyper::Error>> {
    let api_key: &'static str = env!("OWM_KEY");

    let timeout = Timeout::new(Duration::from_secs(TIMEOUT), &handle).unwrap();
    let client = Client::configure().build(&handle);
    let url = format!("http://api.openweathermap.org/data/2.5/weather?q={loc}&APPID={key}&units=metric", loc=q, key=api_key);
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
                "Client timed out while connecting",
            )))
        }
        Err(Either::A((get_error, _timeout))) => Err(get_error),
        Err(Either::B((timeout_error, _get))) => Err(From::from(timeout_error)),
    });
    Box::new(resp)
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_performs_request_to_api() {
        let mut core = tokio_core::reactor::Core::new().unwrap();
        let handle = core.handle();
        let work = current(&handle, String::from("ufa"));
        let r = core.run(work).unwrap();
        let code: i32 = serde_json::from_value(r["cod"].clone()).unwrap();

        assert_eq!(code, 200);
    }

    #[test]
    fn it_handles_wrong_cities() {
        let mut core = tokio_core::reactor::Core::new().unwrap();
        let handle = core.handle();
        let work = current(&handle, String::from("new-ork"));
        let r = core.run(work).unwrap();

        let code: String = serde_json::from_value(r["cod"].clone()).unwrap();

        assert_eq!(code, "404");
    }
}
