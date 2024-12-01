//use std::env;
use std::net::SocketAddr;
//use std::str::Bytes;
//

use std::cell::Cell;
use std::rc::Rc;

use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::thread;

use tokio::io::{self, AsyncWriteExt};

//use bytes::Bytes;
use http_body_util::{
    BodyExt,
    Full,
    BodyDataStream,
    BodyStream,
    StreamBody,
};
use hyper::server::conn::{
    http1,
    http2,
};
use hyper::service::service_fn;
use hyper::{
    body::{
        Incoming as IncomingBody,
        Body as HyperBody,
        Frame,
        Bytes,
    },
    header, Method, Request, Response, StatusCode
};

use hyper_tls::HttpsConnector;


use tokio::net::{TcpListener, TcpStream};

use hyper_util::{
    rt::tokio::{
        TokioIo,
        TokioExecutor,
    },
    client::legacy::Client,
};

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;
type ResultIO<T> = std::result::Result<T, std::io::Error>;

use http_body_util::combinators::BoxBody;


async fn request(
        url: String,
        method: Method,
        headers: hyper::HeaderMap,
        req_data: Full<Bytes>,
        api_key: String,
    ) ->  Result<Response<IncomingBody>> {
    
    let uri = url.parse::<hyper::Uri>().unwrap();

    let authority = uri.authority().unwrap().clone();

    let content_type = headers.get("content-type").unwrap().to_str().unwrap();

    //let auth_header_val = format!("Bearer {}", api_key);
    let auth_header_val = format!("{}", api_key);

    let body = BodyStream::new(req_data);

    let https = HttpsConnector::new();
    let client = Client::builder(TokioExecutor::new()).build::<_, BodyStream<Full<Bytes>>>(https);

    let req = Request::builder()
        .method(method)
        .uri(url)
        .header(hyper::header::HOST, authority.as_str())
        .header(header::CONTENT_TYPE, content_type)
        .header("Authorization", &auth_header_val)
        .body(body)
        .unwrap()
        ;

    //println!("Request proxy: {:?}", req);
    req.headers().iter().for_each(|(name, value)| {
        println!("new req: {}: {}", name.as_str(), value.to_str().unwrap());
    });

    let res = client.request(req).await?;
    return Ok(res);
}

async fn handler(req: Request<IncomingBody>) -> Result<Response<IncomingBody>> {

    //println!("Request Raw: {:?}", req);
    req.headers().iter().for_each(|(name, value)| {
        println!("raw req: {}: {}", name.as_str(), value.to_str().unwrap());
    });

    let api_key = req.headers().get("Authorization").unwrap().to_str().unwrap().to_string();

    let uri_path = req.uri().path();

    let uri = format!("https://api.openai.com/{}", uri_path).to_string();

    let method = req.method().clone();
    let headers = req.headers().clone();
    
    let whole_body = req.into_body().collect().await?.to_bytes();
    let full_body = Full::new(whole_body);

    let res = request(
        uri,
        method,
        headers,
        full_body,
        api_key,
    ).await?;

    res.headers().iter().for_each(|(name, value)| {
        println!("res {}: {}", name.as_str(), value.to_str().unwrap());
    });

    return Ok(res)

}


#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    let addr: SocketAddr = "0.0.0.0:8002".parse().unwrap();

    let listener = TcpListener::bind(&addr).await?;
    println!("Listening on http://{}", addr);
    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        tokio::task::spawn(async move {
            let service = service_fn(move |req| {
                handler(req)
            });

            if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                println!("Failed to serve connection: {:?}", err);
            }
        });
    };
}



