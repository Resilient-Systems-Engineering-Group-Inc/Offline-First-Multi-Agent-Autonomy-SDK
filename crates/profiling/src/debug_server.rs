//! Simple HTTP debug server for live inspection.

use hyper::{Body, Request, Response, Server, Method, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use std::convert::Infallible;
use std::net::SocketAddr;
use tracing::info;

/// Run the debug server on the given address.
pub async fn run(addr: SocketAddr) {
    info!("Starting debug server on http://{}", addr);

    let make_svc = make_service_fn(|_conn| async {
        Ok::<_, Infallible>(service_fn(handle_request))
    });

    let server = Server::bind(&addr).serve(make_svc);

    if let Err(e) = server.await {
        eprintln!("Debug server error: {}", e);
    }
}

async fn handle_request(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let path = req.uri().path();
    let method = req.method();

    match (method, path) {
        (&Method::GET, "/health") => Ok(Response::new(Body::from("OK"))),
        (&Method::GET, "/metrics") => handle_metrics().await,
        (&Method::GET, "/snapshot") => handle_snapshot().await,
        (&Method::GET, "/") => Ok(Response::new(Body::from(
            "<html><body><h1>Debug Server</h1>\
             <ul>\
             <li><a href='/health'>/health</a></li>\
             <li><a href='/metrics'>/metrics</a></li>\
             <li><a href='/snapshot'>/snapshot</a></li>\
             </ul></body></html>"
        ))),
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Not Found"))
            .unwrap()),
    }
}

async fn handle_metrics() -> Result<Response<Body>, Infallible> {
    // In a real implementation, we would collect metrics from the metrics crate.
    let body = "metrics endpoint (not implemented yet)";
    Ok(Response::new(Body::from(body)))
}

async fn handle_snapshot() -> Result<Response<Body>, Infallible> {
    // In a real implementation, we would collect snapshots from agents.
    let body = "snapshot endpoint (not implemented yet)";
    Ok(Response::new(Body::from(body)))
}