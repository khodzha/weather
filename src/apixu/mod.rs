// apixu.org api

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
    let api_key: &'static str = env!("APIXU_KEY");
    let url = format!("http://api.apixu.com/v1/current.json?key={key}&q={loc}", loc=q, key=api_key);

    let resp = async_request(handle, url).and_then(|s| {
        let json_temp: Value = s["current"]["temp_c"].clone();
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

pub fn forecast(handle: &Handle, q: String) -> Box<Future<Item = Vec<f32>, Error = hyper::Error>> {
    let api_key: &'static str = env!("APIXU_KEY");
    let url = format!("http://api.apixu.com/v1/forecast.json?key={key}&q={loc}&days=5", loc=q, key=api_key);

    let resp = async_request(handle, url).and_then(|s| {
        let fcast: Value = s["forecast"]["forecastday"].clone();
        let json_temps: Vec<Value> = serde_json::from_value(fcast).map_err(|e|
            io::Error::new(
                io::ErrorKind::Other,
                e
            )
        )?;
        let temps = json_temps.into_iter().map(|v|
            serde_json::from_value::<f32>(v["day"]["avgtemp_c"].clone()).unwrap()
        ).take(5).collect();
        Ok(temps)
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
