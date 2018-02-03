// apixu.org api

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

pub fn forecast(handle: &Handle, q: String) -> Box<Future<Item = Vec<Option<f32>>, Error = hyper::Error>> {
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
            serde_json::from_value::<f32>(v["day"]["avgtemp_c"].clone()).ok()
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
    fn current_handles_wrong_cities() {
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

    #[test]
    fn forecast_handles_proper_response() {
        let m3 = mock("GET", Matcher::Regex(r#"^/forecast.json.*Perm.*"#.to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"location":{"name":"Perm","region":"Perm'","country":"Russia","lat":58.0,"lon":56.25,"tz_id":"Asia/Yekaterinburg","localtime_epoch":1517670734,"localtime":"2018-02-03 20:12"},"current":{"last_updated_epoch":1517670105,"last_updated":"2018-02-03 20:01","temp_c":-5.0,"temp_f":23.0,"is_day":0,"condition":{"text":"Light snow","icon":"//cdn.apixu.com/weather/64x64/night/326.png","code":1213},"wind_mph":6.9,"wind_kph":11.2,"wind_degree":160,"wind_dir":"SSE","pressure_mb":1026.0,"pressure_in":30.8,"precip_mm":0.0,"precip_in":0.0,"humidity":79,"cloud":0,"feelslike_c":-9.6,"feelslike_f":14.7,"vis_km":10.0,"vis_miles":6.0},"forecast":{"forecastday":[{"date":"2018-02-03","date_epoch":1517616000,"day":{"maxtemp_c":-5.7,"maxtemp_f":21.7,"mintemp_c":-13.0,"mintemp_f":8.6,"avgtemp_c":-9.8,"avgtemp_f":14.4,"maxwind_mph":8.9,"maxwind_kph":14.4,"totalprecip_mm":0.2,"totalprecip_in":0.01,"avgvis_km":13.4,"avgvis_miles":8.0,"avghumidity":92.0,"condition":{"text":"Light freezing rain","icon":"//cdn.apixu.com/weather/64x64/day/311.png","code":1198},"uv":0.5},"astro":{"sunrise":"09:15 AM","sunset":"05:44 PM","moonrise":"09:48 PM","moonset":"10:33 AM"}},{"date":"2018-02-04","date_epoch":1517702400,"day":{"maxtemp_c":-4.5,"maxtemp_f":23.9,"mintemp_c":-8.6,"mintemp_f":16.5,"avgtemp_c":-6.8,"avgtemp_f":19.7,"maxwind_mph":12.8,"maxwind_kph":20.5,"totalprecip_mm":0.4,"totalprecip_in":0.02,"avgvis_km":15.5,"avgvis_miles":9.0,"avghumidity":88.0,"condition":{"text":"Light freezing rain","icon":"//cdn.apixu.com/weather/64x64/day/311.png","code":1198},"uv":0.5},"astro":{"sunrise":"09:13 AM","sunset":"05:46 PM","moonrise":"11:09 PM","moonset":"10:52 AM"}},{"date":"2018-02-05","date_epoch":1517788800,"day":{"maxtemp_c":-1.8,"maxtemp_f":28.8,"mintemp_c":-7.0,"mintemp_f":19.4,"avgtemp_c":-4.2,"avgtemp_f":24.5,"maxwind_mph":13.0,"maxwind_kph":20.9,"totalprecip_mm":0.1,"totalprecip_in":0.0,"avgvis_km":18.3,"avgvis_miles":11.0,"avghumidity":84.0,"condition":{"text":"Light freezing rain","icon":"//cdn.apixu.com/weather/64x64/day/311.png","code":1198},"uv":0.4},"astro":{"sunrise":"09:11 AM","sunset":"05:48 PM","moonrise":"No moonrise","moonset":"11:09 AM"}},{"date":"2018-02-06","date_epoch":1517875200,"day":{"maxtemp_c":-4.0,"maxtemp_f":24.8,"mintemp_c":-9.1,"mintemp_f":15.6,"avgtemp_c":-5.8,"avgtemp_f":21.7,"maxwind_mph":8.1,"maxwind_kph":13.0,"totalprecip_mm":1.8,"totalprecip_in":0.07,"avgvis_km":15.8,"avgvis_miles":9.0,"avghumidity":92.0,"condition":{"text":"Light snow","icon":"//cdn.apixu.com/weather/64x64/day/326.png","code":1213},"uv":0.6},"astro":{"sunrise":"09:08 AM","sunset":"05:51 PM","moonrise":"12:27 AM","moonset":"11:27 AM"}},{"date":"2018-02-07","date_epoch":1517961600,"day":{"maxtemp_c":-4.8,"maxtemp_f":23.4,"mintemp_c":-11.2,"mintemp_f":11.8,"avgtemp_c":-7.6,"avgtemp_f":18.4,"maxwind_mph":6.5,"maxwind_kph":10.4,"totalprecip_mm":1.0,"totalprecip_in":0.04,"avgvis_km":16.8,"avgvis_miles":10.0,"avghumidity":92.0,"condition":{"text":"Light snow","icon":"//cdn.apixu.com/weather/64x64/day/326.png","code":1213},"uv":0.5},"astro":{"sunrise":"09:06 AM","sunset":"05:53 PM","moonrise":"01:43 AM","moonset":"11:45 AM"}}]}}"#)
            .create();

        let mut core = tokio_core::reactor::Core::new().unwrap();
        let handle = core.handle();
        let work = forecast(&handle, String::from("Perm"));
        let r = core.run(work);

        assert_eq!(r.unwrap(), vec![Some(-9.8), Some(-6.8), Some(-4.2), Some(-5.8), Some(-7.6)]);
        m3.assert();
    }

    #[test]
    fn forecast_handles_wrong_cities() {
        let m4 = mock("GET", Matcher::Regex(r#"^/forecast.json.*new-ork.*"#.to_string()))
            .with_status(400)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error":{"code":1006,"message":"No matching location found."}}"#)
            .create();

        let mut core = tokio_core::reactor::Core::new().unwrap();
        let handle = core.handle();
        let work = forecast(&handle, String::from("new-ork"));
        let r = core.run(work);

        assert_eq!(r.unwrap(), vec![None; 5]);
        m4.assert();
    }

    #[test]
    fn forecast_fills_values_upto_five_with_none() {
        let m = mock("GET", Matcher::Regex(r#"^/forecast.json.*Perm.*"#.to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"location":{"name":"Perm","region":"Perm'","country":"Russia","lat":58.0,"lon":56.25,"tz_id":"Asia/Yekaterinburg","localtime_epoch":1517670927,"localtime":"2018-02-03 20:15"},"current":{"last_updated_epoch":1517670105,"last_updated":"2018-02-03 20:01","temp_c":-5.0,"temp_f":23.0,"is_day":0,"condition":{"text":"Light snow","icon":"//cdn.apixu.com/weather/64x64/night/326.png","code":1213},"wind_mph":6.9,"wind_kph":11.2,"wind_degree":160,"wind_dir":"SSE","pressure_mb":1026.0,"pressure_in":30.8,"precip_mm":0.0,"precip_in":0.0,"humidity":79,"cloud":0,"feelslike_c":-9.6,"feelslike_f":14.7,"vis_km":10.0,"vis_miles":6.0},"forecast":{"forecastday":[{"date":"2018-02-03","date_epoch":1517616000,"day":{"maxtemp_c":-5.7,"maxtemp_f":21.7,"mintemp_c":-13.0,"mintemp_f":8.6,"avgtemp_c":-9.8,"avgtemp_f":14.4,"maxwind_mph":8.9,"maxwind_kph":14.4,"totalprecip_mm":0.2,"totalprecip_in":0.01,"avgvis_km":13.4,"avgvis_miles":8.0,"avghumidity":92.0,"condition":{"text":"Light freezing rain","icon":"//cdn.apixu.com/weather/64x64/day/311.png","code":1198},"uv":0.5},"astro":{"sunrise":"09:15 AM","sunset":"05:44 PM","moonrise":"09:48 PM","moonset":"10:33 AM"}},{"date":"2018-02-04","date_epoch":1517702400,"day":{"maxtemp_c":-4.5,"maxtemp_f":23.9,"mintemp_c":-8.6,"mintemp_f":16.5,"avgtemp_c":-6.8,"avgtemp_f":19.7,"maxwind_mph":12.8,"maxwind_kph":20.5,"totalprecip_mm":0.4,"totalprecip_in":0.02,"avgvis_km":15.5,"avgvis_miles":9.0,"avghumidity":88.0,"condition":{"text":"Light freezing rain","icon":"//cdn.apixu.com/weather/64x64/day/311.png","code":1198},"uv":0.5},"astro":{"sunrise":"09:13 AM","sunset":"05:46 PM","moonrise":"11:09 PM","moonset":"10:52 AM"}},{"date":"2018-02-05","date_epoch":1517788800,"day":{"maxtemp_c":-1.8,"maxtemp_f":28.8,"mintemp_c":-7.0,"mintemp_f":19.4,"avgtemp_c":-4.2,"avgtemp_f":24.5,"maxwind_mph":13.0,"maxwind_kph":20.9,"totalprecip_mm":0.1,"totalprecip_in":0.0,"avgvis_km":18.3,"avgvis_miles":11.0,"avghumidity":84.0,"condition":{"text":"Light freezing rain","icon":"//cdn.apixu.com/weather/64x64/day/311.png","code":1198},"uv":0.4},"astro":{"sunrise":"09:11 AM","sunset":"05:48 PM","moonrise":"No moonrise","moonset":"11:09 AM"}}]}}"#)
            .create();

        let mut core = tokio_core::reactor::Core::new().unwrap();
        let handle = core.handle();
        let work = forecast(&handle, String::from("Perm"));
        let r = core.run(work);

        assert_eq!(r.unwrap(), vec![Some(-9.8), Some(-6.8), Some(-4.2), None, None]);
        m.assert();
    }
}
