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

impl Service for WeatherServer {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    fn call(&self, req: Request) -> Self::Future {
        match (req.method(), req.path()) {
            (&Get, "/current") => {
                match req.query() {
                    Some(query) if query.len() > 0 => {
                        let async_apixu = apixu::current(&self.handle, query.to_string());
                        let async_owm = owm::current(&self.handle, query.to_string());
                        let async_wb = weatherbit::current(&self.handle, query.to_string());

                        let resp = futures::future::join_all(vec![async_owm, async_apixu, async_wb]).map(|temps| {
                            let values: Vec<f32> = temps.into_iter().filter_map(|v| v).collect();
                            let sum: f32 = values.iter().sum::<f32>();
                            let avg = sum / values.len() as f32;

                            let body = format!("avg: {:?}°C\n", avg);

                            Response::new()
                                    .with_header(ContentLength(body.len() as u64))
                                    .with_header(ContentType::plaintext())
                                    .with_body(body)
                        });

                        Box::new(resp)
                    }
                    _ => {
                        let body = "Provide location as query";
                        let resp = Response::new()
                                    .with_status(StatusCode::UnprocessableEntity)
                                    .with_header(ContentLength(body.len() as u64))
                                    .with_header(ContentType::plaintext())
                                    .with_body(body);
                        Box::new(futures::future::ok(resp))
                    }
                }
            },
            (&Get, "/forecast") => {
                match req.query() {
                    None => {
                        let mut resp = Response::new();
                        resp.set_status(StatusCode::UnprocessableEntity);
                        Box::new(futures::future::ok(resp))
                    }
                    Some(query) => {
                        let async_apixu = apixu::forecast(&self.handle, query.to_string());
                        let async_wb = weatherbit::forecast(&self.handle, query.to_string());

                        let body = async_apixu.join(async_wb).map(|(apixu_temp, wb_temp)| {
                            let r = format!("apixu: {:?}°C\nwb: {:?}°C\n", apixu_temp, wb_temp);
                            Response::new()
                                    .with_header(ContentLength(r.len() as u64))
                                    .with_header(ContentType::plaintext())
                                    .with_body(r)
                        });

                        Box::new(body)
                    }
                }
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
