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

pub type ResponseStream = Box<Stream<Item = String, Error=Error>>;

mod async_request;
mod owm;
mod apixu;
mod weatherbit;

struct WeatherServer{
    handle: Handle
}
impl WeatherServer {
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

        match (req.method(), req.path()) {
            (&Get, "/current") => {
                if query.len() == 0 {
                    return Self::empty_query_body();
                }

                let async_apixu = apixu::current(&self.handle, query.to_string());
                let async_owm = owm::current(&self.handle, query.to_string());
                let async_wb = weatherbit::current(&self.handle, query.to_string());

                let resp = futures::future::join_all(vec![async_owm, async_apixu, async_wb]).map(|temps| {
                    let values: Vec<f32> = temps.into_iter().filter_map(|v| v).collect();

                    let sum: f32 = values.iter().sum::<f32>();
                    let avg = sum / values.len() as f32;

                    let body = if values.len() > 0 {
                        format!("avg: {:.1}°C\n", avg)
                    } else {
                        format!("Failed to receive APIs responses")
                    };

                    Response::new()
                            .with_header(ContentLength(body.len() as u64))
                            .with_header(ContentType::plaintext())
                            .with_body(body)
                });

                Box::new(resp)

            },
            (&Get, "/forecast") => {
                if query.len() == 0 {
                    return Self::empty_query_body();
                }

                let async_apixu = apixu::forecast(&self.handle, query.to_string());
                let async_wb = weatherbit::forecast(&self.handle, query.to_string());

                let body = async_apixu.join(async_wb).map(|(apixu_temp, wb_temp)| {
                    let avg_temps: Vec<Option<f32>> = apixu_temp.iter().zip(wb_temp.iter()).map(|(&a, &b)| match (a, b) {
                        (Some(x), Some(y)) => Some((x+y) / 2.0),
                        (Some(x), None) => Some(x),
                        (None, Some(y)) => Some(y),
                        _ => None
                    }).collect();

                    let r = Self::format_temps(avg_temps);
                    Response::new()
                            .with_header(ContentLength(r.len() as u64))
                            .with_header(ContentType::plaintext())
                            .with_body(r)
                });

                Box::new(body)
            },
            _ => {
                let mut resp = Response::new();
                resp.set_status(StatusCode::NotFound);
                Box::new(futures::future::ok(resp))
            }
        }
    }
}

fn main() {
    let addr = "127.0.0.1:1337".parse().unwrap();

    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();
    let client_handle = core.handle();

    let serve = Http::new().serve_addr_handle(&addr, &handle, move || Ok(WeatherServer{handle: client_handle.clone()})).unwrap();
    println!("Listening on http://{} with 1 thread.", serve.incoming_ref().local_addr());

    let h2 = handle.clone();
    handle.spawn(serve.for_each(move |conn| {
        h2.spawn(conn.map(|_| ()).map_err(|err| println!("serve error: {:?}", err)));
        Ok(())
    }).map_err(|_| ()));

    core.run(futures::future::empty::<(), ()>()).unwrap();
}
