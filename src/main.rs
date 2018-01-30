extern crate futures;
extern crate hyper;
extern crate tokio_core;
extern crate serde_json;

pub mod owm;

use futures::{Future, Stream};
use hyper::{Get, StatusCode};
use hyper::error::Error;
use tokio_core::reactor::Handle;
use hyper::header::ContentLength;
use hyper::server::{Http, Service, Request, Response};

pub type ResponseStream = Box<Stream<Item = String, Error=Error>>;

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
            (&Get, "/") => {
                match req.query() {
                    None => {
                        let mut resp = Response::new();
                        resp.set_status(StatusCode::UnprocessableEntity);
                        Box::new(futures::future::ok(resp))
                    }
                    Some(query) => {
                        let body_box = owm::current(&self.handle, query.to_string()).map(|temp| {
                            let r = format!("response is: {}", temp);
                            Response::new()
                                    .with_header(ContentLength(r.len() as u64))
                                    .with_body(r)
                        });
                        Box::new(body_box)
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
