extern crate diesel;

extern crate dotenv;

extern crate futures;

extern crate handlebars;

extern crate hyper;
extern crate reqwest;

extern crate serde;
extern crate serde_json;

use hyper::StatusCode;
use hyper::header::{ContentType, Location};
use hyper::server::{Http, Service, Request, Response};

use std::collections::HashMap;
use std::io::prelude::*;
use std::fs::File;
use std::net::SocketAddr;
use std::path::Path;

pub struct Contributors {
    routes: HashMap<String, Box<Fn(Request) -> ::futures::Finished<Response, hyper::Error> + Sync + Send + 'static>>,
}

impl Contributors {
    pub fn new() -> Contributors {
        Contributors {
            routes: HashMap::new(),
        }
    }

    pub fn add_route<F>(&mut self, path: &str, f: F) 
        where F: Fn(Request) -> ::futures::Finished<Response, hyper::Error> + Sync + Send + 'static
    {
        self.routes.insert(path.to_string(), Box::new(f));
    }
}

impl Service for Contributors {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = ::futures::Finished<Response, hyper::Error>;

    fn call(&self, req: Request) -> Self::Future {
        // redirect to ssl
        // from http://jaketrent.com/post/https-redirect-node-heroku/
        println!("request!");
        if let Some(raw) = req.headers().get_raw("x-forwarded-proto") {
            println!("seen header: {:?}", raw);
            if raw != &b"https"[..] {
                return ::futures::finished(
                    Response::new()
                    .with_header(Location(format!("https://thanks.rust-lang.org{}", req.path())))
                    .with_status(StatusCode::MovedPermanently)
                );
            }
        }

        // first, we serve static files
        let path = req.path().to_string();

        println!("PATH: {:?}", path);

        // ... you trying to do something bad?
        if path.contains("./") || path.contains("../") {
            // GET OUT
            return ::futures::finished(Response::new()
                .with_header(ContentType::html())
                .with_status(StatusCode::NotFound));
        }

        if path.starts_with("/public") && Path::new(&path[1..]).exists() {
            println!("serve static arm\npath: {}", path);
            let mut f = File::open(&path[1..]).unwrap();
            let mut source = Vec::new();
            f.read_to_end(&mut source).unwrap();

            return ::futures::finished(Response::new()
              .with_body(source));
        }

        // next, we check routes
        
        let handler = if let Some(h) = self.routes.get(req.path()) {
            h
        } else if let Some(h) = self.routes.get("*") {
            // * is the catch all route
            h
        } else {
            // if we get here, we didn't find anything
            return ::futures::finished(Response::new()
                .with_header(ContentType::html())
                .with_status(StatusCode::NotFound));
        };

        handler(req)
    }
}

pub struct Server;

impl Server {
    pub fn run(&self, addr: &SocketAddr, contributors: Contributors) {
        let a = std::sync::Arc::new(contributors);

        let server = Http::new().bind(addr, move || Ok(a.clone())).unwrap();

        server.run().unwrap();
    }
}
