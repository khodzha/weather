// openweathermap.org api

extern crate futures;
extern crate hyper;
extern crate tokio_core;
extern crate serde_json;

use self::serde_json::Value;
use std::io;
use self::futures::{Future};

use self::tokio_core::reactor::{Handle};
use async_request::async_request;

pub fn current(handle: &Handle, q: String) -> Box<Future<Item = f32, Error = hyper::Error>> {
    let api_key: &'static str = env!("OWM_KEY");
    let url = format!("http://api.openweathermap.org/data/2.5/weather?q={loc}&APPID={key}&units=metric", loc=q, key=api_key);

    let resp = async_request(handle, url).and_then(|s| {
        let json_temp: Value = s["main"]["temp"].clone();
        let temp = serde_json::from_value::<f32>(json_temp).map_err(|e|
            io::Error::new(
                io::ErrorKind::Other,
                e
            )
        )?;
        Ok(temp)
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
        let r = core.run(work);

        assert!(r.is_ok());
    }

    #[test]
    fn it_handles_wrong_cities() {
        let mut core = tokio_core::reactor::Core::new().unwrap();
        let handle = core.handle();
        let work = current(&handle, String::from("new-ork"));
        let r = core.run(work);

        assert!(r.is_err());
    }
}
