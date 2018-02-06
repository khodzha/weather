extern crate futures;
extern crate hyper;
extern crate tokio_core;
extern crate serde_json;

use futures::{Future, Stream};
use hyper::{Get, StatusCode};
use hyper::error::Error;
use tokio_core::reactor::Handle;
use hyper::header::{ContentLength, ContentType};
use hyper::server::{Http, Service, Request, Response};

use async_request::error::ApiError;

pub type ResponseStream = Box<Stream<Item = String, Error=Error>>;

pub mod async_request;
mod owm;
mod apixu;
mod weatherbit;

pub fn start_server(address: &str, keys: ApiKeys) -> tokio_core::reactor::Core {
    let addr = address.parse().unwrap();

    let core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();
    let client_handle = core.handle();

    let serve = Http::new().serve_addr_handle(&addr, &handle, move || Ok(WeatherServer::new(
        client_handle.clone(),
        keys.clone()
    ))).unwrap();
    println!("Listening on http://{} with 1 thread.", serve.incoming_ref().local_addr());

    let h2 = handle.clone();
    handle.spawn(serve.for_each(move |conn| {
        h2.spawn(conn.map(|_| ()).map_err(|err| println!("serve error: {:?}", err)));
        Ok(())
    }).map_err(|_| ()));

    core
}


pub struct WeatherServer {
    handle: Handle,
    keys: ApiKeys
}

#[derive(Debug, Clone)]
pub struct ApiKeys {
    owm_key: String,
    apixu_key: String,
    weatherbit_key: String
}

impl ApiKeys {
    pub fn new(owm_key: String, apixu_key: String, weatherbit_key: String) -> ApiKeys {
        ApiKeys {
            owm_key: owm_key,
            apixu_key: apixu_key,
            weatherbit_key: weatherbit_key
        }
    }
}

impl WeatherServer {
    pub fn new(handle: Handle, keys: ApiKeys) -> WeatherServer {
        WeatherServer {
            handle: handle,
            keys: keys,
        }
    }

    fn empty_query_body() -> <WeatherServer as Service>::Future {
        let body = "Provide location as query";
        let resp = Response::new()
                    .with_status(StatusCode::UnprocessableEntity)
                    .with_header(ContentLength(body.len() as u64))
                    .with_header(ContentType::plaintext())
                    .with_body(body);
        Box::new(futures::future::ok(resp))
    }

    fn format_temps(vec: Vec<Option<f32>>) -> String {
        let new_vec: Vec<String> = vec.into_iter().enumerate().map(|(idx, val)| match val {
            Some(t) => format!("day{} temp = {:.1}°C", idx+1, t),
            None => format!("day{} temp is unknown°C", idx+1)
        }).collect();
        new_vec.join("\n")
    }
}

impl Service for WeatherServer {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    fn call(&self, req: Request) -> Self::Future {

        let query = match req.query() {
            Some(s) => s,
            None => ""
        };

        let mut status = StatusCode::Ok;

        match (req.method(), req.path()) {
            (&Get, "/") => {
                let body = "<html> \
                    <body> \
                    examples:<br/> \
                    <a href='/current?tomsk'>current weather in tomsk</a><br/> \
                    <a href='/forecast?perm'>5 days forecast for perm</a> \
                    </body> \
                    </html> \
                ";
                let resp = futures::future::ok(
                                Response::new()
                                    .with_header(ContentLength(body.len() as u64))
                                    .with_header(ContentType::html())
                                    .with_status(StatusCode::Ok)
                                    .with_body(body)
                            );

                Box::new(resp)
            },
            (&Get, "/current") => {
                if query.len() == 0 {
                    return Self::empty_query_body();
                }

                let async_apixu = apixu::current(&self.handle, query, &self.keys.apixu_key);
                let async_owm = owm::current(&self.handle, query, &self.keys.owm_key);
                let async_wb = weatherbit::current(&self.handle, query, &self.keys.weatherbit_key);

                let resp = futures::future::join_all(vec![async_owm, async_apixu, async_wb]).map(move |temps| {
                    let body = if temps.iter().all(|v| v.is_err()) {
                        if temps.into_iter().all(|v| v == Err(ApiError::LocationNotFound)) {
                            status = StatusCode::NotFound;
                            format!("Location not found")
                        } else {
                            status = StatusCode::InternalServerError;
                            format!("Something went wrong")
                        }
                    } else {
                        let values: Vec<f32> = temps.into_iter().filter_map(|v| v.ok()).collect();

                        let sum: f32 = values.iter().sum::<f32>();
                        let avg = sum / values.len() as f32;

                        if values.len() > 0 {
                            format!("avg: {:.1}°C\n", avg)
                        } else {
                            format!("Failed to receive APIs responses")
                        }
                    };


                    Response::new()
                            .with_header(ContentLength(body.len() as u64))
                            .with_header(ContentType::plaintext())
                            .with_status(status)
                            .with_body(body)
                });

                Box::new(resp)

            },
            (&Get, "/forecast") => {
                if query.len() == 0 {
                    return Self::empty_query_body();
                }

                let async_apixu = apixu::forecast(&self.handle, query, &self.keys.apixu_key);
                let async_wb = weatherbit::forecast(&self.handle, query, &self.keys.weatherbit_key);

                let resp = async_apixu.join(async_wb).map(move |(apixu_temp, wb_temp)| {
                    let body = if apixu_temp.is_err() && wb_temp.is_err() {
                        if apixu_temp == Err(ApiError::LocationNotFound) && wb_temp == Err(ApiError::LocationNotFound) {
                            status = StatusCode::NotFound;
                            format!("Location not found")
                        } else {
                            status = StatusCode::InternalServerError;
                            format!("Something went wrong")
                        }
                    } else {
                        let apixu_temp_vec = apixu_temp.unwrap_or(vec![None; 5]);
                        let wb_temp_vec = wb_temp.unwrap_or(vec![None; 5]);

                        let avg_temps: Vec<Option<f32>> = apixu_temp_vec.iter().zip(wb_temp_vec.iter()).map(|(&a, &b)| match (a, b) {
                            (Some(x), Some(y)) => Some((x+y) / 2.0),
                            (Some(x), None) => Some(x),
                            (None, Some(y)) => Some(y),
                            _ => None
                        }).collect();

                        Self::format_temps(avg_temps)
                    };


                    Response::new()
                            .with_header(ContentLength(body.len() as u64))
                            .with_header(ContentType::plaintext())
                            .with_status(status)
                            .with_body(body)
                });

                Box::new(resp)
            },
            _ => {
                let mut resp = Response::new();
                resp.set_status(StatusCode::NotFound);
                Box::new(futures::future::ok(resp))
            }
        }
    }
}
