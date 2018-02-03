extern crate futures;
extern crate weather;
extern crate tokio_core;

use futures::Future;
use std::env::var;
use weather::async_request::async_request;

#[test]
fn it_works() {
    let weatherbit_key = match var("WEATHERBIT_KEY") {
        Ok(v) => v,
        Err(e) => panic!("WEATHERBIT_KEY is absent, {:?}", e),
    };

    let apixu_key = match var("APIXU_KEY") {
        Ok(v) => v,
        Err(e) => panic!("APIXU_KEY is absent, {:?}", e),
    };
    let owm_key = match var("OWM_KEY") {
        Ok(v) => v,
        Err(e) => panic!("OWM_KEY is absent, {:?}", e),
    };

    let keys = weather::ApiKeys::new(owm_key, apixu_key, weatherbit_key);
    let mut core = weather::start_server("0.0.0.0:13337", keys);

    let response_future = async_request(&core.handle(), "http://0.0.0.0:13337/current?Tomsk").map(|f| {
        f.iter().cloned().collect::<Vec<u8>>()
    });

    let resp = core.run(response_future).unwrap();
    let str_body = std::str::from_utf8(&resp).unwrap();
    assert!(str_body.contains("avg"));
}
