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

#[cfg(not(test))]
const API_ROOT: &'static str = "http://api.openweathermap.org/data/2.5";

pub fn current(handle: &Handle, q: &str, api_key: &str) -> Box<Future<Item = Option<f32>, Error = hyper::Error>> {
    let url = format!("{api_root}/weather?q={loc}&APPID={key}&units=metric", loc=q, key=api_key, api_root=API_ROOT);

    let resp = async_request(handle, url).and_then(|s| {
        let json_temp: Value = s["main"]["temp"].clone();
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
        let m1 = mock("GET", Matcher::Regex(r#"^/weather.*Yakutsk.*"#.to_string()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"coord":{"lon":129.73,"lat":62.03},"weather":[{"id":741,"main":"Fog","description":"fog","icon":"50n"}],"base":"stations","main":{"temp":-41,"pressure":1038,"humidity":57,"temp_min":-41,"temp_max":-41},"visibility":200,"wind":{"speed":1.27,"deg":10.0068},"clouds":{"all":8},"dt":1517661000,"sys":{"type":1,"id":7228,"message":0.0059,"country":"RU","sunrise":1517614868,"sunset":1517642978},"id":2013159,"name":"Yakutsk","cod":200}"#)
            .create();

        let mut core = tokio_core::reactor::Core::new().unwrap();
        let handle = core.handle();
        let work = current(&handle, "Yakutsk", "");
        let r = core.run(work);

        assert_eq!(r.unwrap(), Some(-41.0));
        m1.assert();
    }

    #[test]
    fn it_handles_wrong_cities() {
        let m2 = mock("GET", Matcher::Regex(r#"^/weather.*new-rk.*"#.to_string()))
            .with_status(404)
            .with_header("content-type", "application/json")
            .with_body(r#"{"cod":"404","message":"city not found"}"#)
            .create();
        let mut core = tokio_core::reactor::Core::new().unwrap();
        let handle = core.handle();
        let work = current(&handle, "new-rk", "");
        let r = core.run(work);

        assert_eq!(r.unwrap(), None);
        m2.assert();
    }
}
