// weatherbit.io api

extern crate futures;
extern crate hyper;
extern crate tokio_core;
extern crate serde_json;

use self::serde_json::Value;
use std::io;
use self::futures::{Future};

use self::tokio_core::reactor::{Handle};
use async_request::async_request;

#[cfg(not(test))]
const API_ROOT: &'static str = "http://api.weatherbit.io/v2.0";

pub fn current(handle: &Handle, q: String) -> Box<Future<Item = Option<f32>, Error = hyper::Error>> {
    let api_key: &'static str = env!("WEATHERBIT_KEY");
    let url = format!("{api_root}/current?key={key}&city={loc}", loc=q, key=api_key, api_root=API_ROOT);

    let resp = async_request(handle, url).and_then(|s| {
        let json_temp: Value = s["data"][0]["temp"].clone();
        let temp = serde_json::from_value::<f32>(json_temp).map_err(|e|
            io::Error::new(
                io::ErrorKind::Other,
                e
            )
        )?;
        Ok(Some(temp))
    }).or_else(|_s|
        Ok(None)
    );

    Box::new(resp)
}

pub fn forecast(handle: &Handle, q: String) -> Box<Future<Item = Vec<f32>, Error = hyper::Error>> {
    let api_key: &'static str = env!("WEATHERBIT_KEY");
    let url = format!("{api_root}/forecast/daily?key={key}&city={loc}&days=5", loc=q, key=api_key, api_root=API_ROOT);

    let resp = async_request(handle, url).and_then(|s| {
        let fcast: Value = s["data"].clone();
        let json_temps: Vec<Value> = serde_json::from_value(fcast).map_err(|e|
            io::Error::new(
                io::ErrorKind::Other,
                e
            )
        )?;
        let temps = json_temps.into_iter().map(|v|
            serde_json::from_value::<f32>(v["temp"].clone()).unwrap()
        ).take(5).collect();
        Ok(temps)
    });

    Box::new(resp)
}

#[cfg(test)]
extern crate mockito;

#[cfg(test)]
const API_ROOT: &'static str = mockito::SERVER_URL;

#[cfg(test)]
mod tests {
    use super::*;
    use self::mockito::{mock, Matcher};

    #[test]
    fn it_performs_request_to_api() {
        let m1 = mock("GET", Matcher::Regex(r#"^/current.*Ufa.*"#.to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":[{"rh":92,"pod":"d","pres":1010.74,"timezone":"Asia\/Yekaterinburg","weather":{"icon":"a05d","code":"741","description":"Fog"},"country_code":"RU","clouds":75,"vis":0.5,"wind_spd":2,"wind_cdir_full":"south-southeast","app_temp":-10.7,"lon":55.96779,"state_code":"08","ts":1517661000,"elev_angle":6,"h_angle":90,"dewpt":-8.2,"ob_time":"2018-02-03 12:30","uv":0,"sunset":"12:59","sunrise":"04:00","city_name":"Ufa","precip":null,"station":"UWUU","lat":54.74306,"dhi":0,"datetime":"2018-02-03:12","temp":-7,"wind_dir":160,"slp":1029,"wind_cdir":"SSE"}],"count":1}"#)
            .create();

        let mut core = tokio_core::reactor::Core::new().unwrap();
        let handle = core.handle();
        let work = current(&handle, String::from("Ufa"));
        let r = core.run(work);

        assert_eq!(r.unwrap(), Some(-7.0));
        m1.assert();
    }

    #[test]
    fn it_handles_wrong_cities() {
        let m2 = mock("GET", Matcher::Regex(r#"^/current.*new-ork.*"#.to_string()))
            .with_status(204)
            .with_header("content-type", "application/json")
            .create();
        let mut core = tokio_core::reactor::Core::new().unwrap();
        let handle = core.handle();
        let work = current(&handle, String::from("new-ork"));
        let r = core.run(work);

        assert_eq!(r.unwrap(), None);
        m2.assert();
    }
}
