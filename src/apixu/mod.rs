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

#[cfg(not(test))]
const API_ROOT: &'static str = "http://api.apixu.com/v1";

pub fn current(handle: &Handle, q: String) -> Box<Future<Item = Option<f32>, Error = hyper::Error>> {
    let api_key: &'static str = env!("APIXU_KEY");
    let url = format!("{api_root}/current.json?key={key}&q={loc}", loc=q, key=api_key, api_root=API_ROOT);

    let resp = async_request(handle, url).and_then(|s| {
        let json_temp: Value = s["current"]["temp_c"].clone();
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
    let api_key: &'static str = env!("APIXU_KEY");
    let url = format!("{api_root}/forecast.json?key={key}&q={loc}&days=5", loc=q, key=api_key, api_root=API_ROOT);

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
extern crate mockito;

#[cfg(test)]
const API_ROOT: &'static str = mockito::SERVER_URL;

#[cfg(test)]
mod tests {
    use super::*;
    use self::mockito::{mock, Matcher};

    #[test]
    fn it_performs_request_to_api() {
        let m1 = mock("GET", Matcher::Regex(r#"^/current.json.*Tomsk.*"#.to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"location":{"name":"Томск","region":"Tomsk","country":"Россия","lat":56.5,"lon":84.97,"tz_id":"Asia/Tomsk","localtime_epoch":1517661995,"localtime":"2018-02-03 19:46"},"current":{"last_updated_epoch":1517661028,"last_updated":"2018-02-03 19:30","temp_c":-14.0,"temp_f":6.8,"is_day":0,"condition":{"text":"Overcast","icon":"//cdn.apixu.com/weather/64x64/night/122.png","code":1009},"wind_mph":0.0,"wind_kph":0.0,"wind_degree":0,"wind_dir":"N","pressure_mb":1032.0,"pressure_in":31.0,"precip_mm":0.0,"precip_in":0.0,"humidity":78,"cloud":0,"feelslike_c":-14.0,"feelslike_f":6.8,"vis_km":5.0,"vis_miles":3.0}}"#)
            .create();

        let mut core = tokio_core::reactor::Core::new().unwrap();
        let handle = core.handle();
        let work = current(&handle, String::from("Tomsk"));
        let r = core.run(work);

        assert_eq!(r.unwrap(), Some(-14.0));
        m1.assert();
    }

    #[test]
    fn it_handles_wrong_cities() {
        let m2 = mock("GET", Matcher::Regex(r#"^/current.json.*new-ork.*"#.to_string()))
            .with_status(400)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error":{"code":1006,"message":"No matching location found."}}"#)
            .create();

        let mut core = tokio_core::reactor::Core::new().unwrap();
        let handle = core.handle();
        let work = current(&handle, String::from("new-ork"));
        let r = core.run(work);

        assert_eq!(r.unwrap(), None);
        m2.assert();
    }
}
