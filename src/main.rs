extern crate weather;
extern crate tokio_core;
extern crate hyper;
extern crate futures;

use std::env::var;

fn main() {
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
    let mut core = weather::start_server("0.0.0.0:1337", keys);
    core.run(futures::future::empty::<(), ()>()).unwrap();
}
