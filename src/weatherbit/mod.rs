// weatherbit.io api

extern crate futures;
extern crate hyper;
extern crate tokio_core;
extern crate serde_json;
extern crate itertools;

use self::serde_json::Value;
use std::io;
use self::futures::{Future};

use self::tokio_core::reactor::{Handle};
use async_request::async_request;

use self::itertools::EitherOrBoth::{Both};
use self::itertools::Itertools;

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

pub fn forecast(handle: &Handle, q: String) -> Box<Future<Item = Vec<Option<f32>>, Error = hyper::Error>> {
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
        let temps: Vec<Option<f32>> = json_temps.into_iter().map(|v|
            serde_json::from_value::<f32>(v["temp"].clone()).ok()
        ).zip_longest(0..5).map(|v| match v {
            Both(Some(t), _) => Some(t),
            _ => None
        }).collect();

        Ok(temps)
    }).or_else(|_s|
        Ok(vec![None; 5])
    );

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
    fn current_performs_request_to_api() {
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
    fn current_handles_wrong_cities() {
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

    #[test]
    fn forecast_handles_proper_response() {
        let m = mock("GET", Matcher::Regex(r#"^/forecast/daily.*Ekaterinburg.*"#.to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":[{"wind_cdir":"SSW","rh":92,"wind_spd":0,"pop":0,"wind_cdir_full":"south-southwest","slp":1031.8,"app_max_temp":-8.3,"pres":995.5,"dewpt":-10.1,"snow":0,"uv":2,"ts":1517659200,"wind_dir":197,"weather":{"icon":"c02d","code":"802","description":"Scattered clouds"},"app_min_temp":-16.3,"max_temp":-6.8,"snow_depth":270,"precip":0,"max_dhi":449.1,"datetime":"2018-02-03","temp":-9,"min_temp":-14,"clouds":32,"vis":7},{"wind_cdir":"SE","rh":92,"wind_spd":0,"pop":0,"wind_cdir_full":"southeast","slp":1033.2,"app_max_temp":-7.8,"pres":996.8,"dewpt":-12,"snow":0,"uv":2,"ts":1517745600,"wind_dir":135,"weather":{"icon":"c03d","code":"803","description":"Broken clouds"},"app_min_temp":-16.1,"max_temp":-6.3,"snow_depth":270,"precip":0,"max_dhi":321.8,"datetime":"2018-02-04","temp":-11,"min_temp":-13.8,"clouds":64,"vis":7},{"wind_cdir":"NE","rh":88,"wind_spd":1,"pop":0,"wind_cdir_full":"northeast","slp":1030.3,"app_max_temp":-5,"pres":994.3,"dewpt":-10.6,"snow":0,"uv":2,"ts":1517832000,"wind_dir":45,"weather":{"icon":"c03d","code":"803","description":"Broken clouds"},"app_min_temp":-15.6,"max_temp":-3.8,"snow_depth":270,"precip":0,"max_dhi":230.3,"datetime":"2018-02-05","temp":-9,"min_temp":-13.3,"clouds":87,"vis":9},{"wind_cdir":"S","rh":87,"wind_spd":1,"pop":10,"wind_cdir_full":"south","slp":1036.4,"app_max_temp":-3,"pres":1000.7,"dewpt":-7.8,"snow":3.54,"uv":2,"ts":1517918400,"wind_dir":180,"weather":{"icon":"c04d","code":"804","description":"Overcast clouds"},"app_min_temp":-12.8,"max_temp":-2,"snow_depth":273.5,"precip":0.26,"max_dhi":182.6,"datetime":"2018-02-06","temp":-6,"min_temp":-10.8,"clouds":99,"vis":9},{"wind_cdir":"SW","rh":82,"wind_spd":1,"pop":0,"wind_cdir_full":"southwest","slp":1046.6,"app_max_temp":-3.9,"pres":1010.6,"dewpt":-9.6,"snow":0,"uv":2,"ts":1518004800,"wind_dir":225,"weather":{"icon":"c03d","code":"803","description":"Broken clouds"},"app_min_temp":-12.2,"max_temp":-2.8,"snow_depth":273.5,"precip":0,"max_dhi":234.3,"datetime":"2018-02-07","temp":-7,"min_temp":-10.3,"clouds":86,"vis":10}],"city_name":"Ekaterinburg","lon":"60.6122","timezone":"Asia\/Yekaterinburg","lat":"56.8519","country_code":"RU","state_code":"71"}"#)
            .create();

        let mut core = tokio_core::reactor::Core::new().unwrap();
        let handle = core.handle();
        let work = forecast(&handle, String::from("Ekaterinburg"));
        let r = core.run(work);

        assert_eq!(r.unwrap(), vec![Some(-9.0), Some(-11.0), Some(-9.0), Some(-6.0), Some(-7.0)]);
        m.assert();
    }

    #[test]
    fn forecast_handles_wrong_cities() {
        let m = mock("GET", Matcher::Regex(r#"^/forecast/daily.*new-ork.*"#.to_string()))
            .with_status(204)
            .create();

        let mut core = tokio_core::reactor::Core::new().unwrap();
        let handle = core.handle();
        let work = forecast(&handle, String::from("new-ork"));
        let r = core.run(work);

        assert_eq!(r.unwrap(), vec![None; 5]);
        m.assert();
    }

    #[test]
    fn forecast_fills_values_upto_five_with_none() {
        let m = mock("GET", Matcher::Regex(r#"^/forecast/daily.*Ekaterinburg.*"#.to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":[{"wind_cdir":"SSW","rh":92,"wind_spd":0,"pop":0,"wind_cdir_full":"south-southwest","slp":1031.8,"app_max_temp":-8.3,"pres":995.5,"dewpt":-10.1,"snow":0,"uv":2,"ts":1517659200,"wind_dir":197,"weather":{"icon":"c02d","code":"802","description":"Scattered clouds"},"app_min_temp":-16.3,"max_temp":-6.8,"snow_depth":270,"precip":0,"max_dhi":449.1,"datetime":"2018-02-03","temp":-9,"min_temp":-14,"clouds":32,"vis":7},{"wind_cdir":"SE","rh":92,"wind_spd":0,"pop":0,"wind_cdir_full":"southeast","slp":1033.2,"app_max_temp":-7.8,"pres":996.8,"dewpt":-12,"snow":0,"uv":2,"ts":1517745600,"wind_dir":135,"weather":{"icon":"c03d","code":"803","description":"Broken clouds"},"app_min_temp":-16.1,"max_temp":-6.3,"snow_depth":270,"precip":0,"max_dhi":321.8,"datetime":"2018-02-04","temp":-11,"min_temp":-13.8,"clouds":64,"vis":7},{"wind_cdir":"NE","rh":88,"wind_spd":1,"pop":0,"wind_cdir_full":"northeast","slp":1030.3,"app_max_temp":-5,"pres":994.3,"dewpt":-10.6,"snow":0,"uv":2,"ts":1517832000,"wind_dir":45,"weather":{"icon":"c03d","code":"803","description":"Broken clouds"},"app_min_temp":-15.6,"max_temp":-3.8,"snow_depth":270,"precip":0,"max_dhi":230.3,"datetime":"2018-02-05","temp":-9,"min_temp":-13.3,"clouds":87,"vis":9}],"city_name":"Ekaterinburg","lon":"60.6122","timezone":"Asia\/Yekaterinburg","lat":"56.8519","country_code":"RU","state_code":"71"}"#)
            .create();

        let mut core = tokio_core::reactor::Core::new().unwrap();
        let handle = core.handle();
        let work = forecast(&handle, String::from("Ekaterinburg"));
        let r = core.run(work);

        assert_eq!(r.unwrap(), vec![Some(-9.0), Some(-11.0), Some(-9.0), None, None]);
        m.assert();
    }
}
