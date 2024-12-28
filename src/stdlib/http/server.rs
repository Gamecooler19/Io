use std::{
    sync::Arc,
    collections::HashMap,
    net::SocketAddr,
};
use hyper::{
    Body, Request, Response, Server,
    service::{make_service_fn, service_fn},
    header, StatusCode,
};
use tokio::sync::RwLock;
use crate::{Result, error::IoError};

pub type HandlerFn = Arc<dyn Fn(Request<Body>) -> Result<Response<Body>> + Send + Sync>;

pub struct HttpServer {
    routes: Arc<RwLock<HashMap<String, HashMap<String, HandlerFn>>>>,
    middleware: Vec<Arc<dyn Middleware>>,
    addr: SocketAddr,
}

#[async_trait::async_trait]
pub trait Middleware: Send + Sync {
    async fn process(
        &self,
        request: Request<Body>,
        next: Box<dyn Fn(Request<Body>) -> Result<Response<Body>> + Send>,
    ) -> Result<Response<Body>>;
}

impl HttpServer {
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            routes: Arc::new(RwLock::new(HashMap::new())),
            middleware: Vec::new(),
            addr,
        }
    }

    pub async fn add_route<F>(&self, method: &str, path: &str, handler: F) -> Result<()>
    where
        F: Fn(Request<Body>) -> Result<Response<Body>> + Send + Sync + 'static,
    {
        let mut routes = self.routes.write().await;
        let method_routes = routes
            .entry(method.to_uppercase())
            .or_insert_with(HashMap::new);

        method_routes.insert(path.to_string(), Arc::new(handler));
        Ok(())
    }

    pub fn add_middleware<M: Middleware + 'static>(&mut self, middleware: M) {
        self.middleware.push(Arc::new(middleware));
    }

    pub async fn start(self) -> Result<()> {
        let routes = self.routes.clone();
        let middleware = self.middleware.clone();

        let make_svc = make_service_fn(move |_| {
            let routes = routes.clone();
            let middleware = middleware.clone();

            async move {
                Ok::<_, hyper::Error>(service_fn(move |req| {
                    let routes = routes.clone();
                    let middleware = middleware.clone();
                    
                    async move {
                        handle_request(req, routes, middleware).await
                            .or_else(|e| Ok::<_, hyper::Error>(create_error_response(e)))
                    }
                }))
            }
        });

        Server::bind(&self.addr)
            .serve(make_svc)
            .await
            .map_err(|e| IoError::runtime_error(format!("Server error: {}", e)))?;

        Ok(())
    }
}

async fn handle_request(
    request: Request<Body>,
    routes: Arc<RwLock<HashMap<String, HashMap<String, HandlerFn>>>>,
    middleware: Vec<Arc<dyn Middleware>>,
) -> Result<Response<Body>> {
    let method = request.method().to_string();
    let path = request.uri().path().to_string();

    let routes_guard = routes.read().await;
    let method_routes = routes_guard.get(&method)
        .ok_or_else(|| IoError::runtime_error("Method not allowed"))?;

    let handler = method_routes.get(&path)
        .ok_or_else(|| IoError::runtime_error("Not found"))?;

    // Apply middleware chain
    let mut current_handler: Box<dyn Fn(Request<Body>) -> Result<Response<Body>> + Send> = 
        Box::new(move |req| handler(req));

    for m in middleware.into_iter().rev() {
        let next = current_handler;
        current_handler = Box::new(move |req| {
            let m = m.clone();
            let next = next.clone();
            Box::pin(async move {
                m.process(req, Box::new(move |req| next(req))).await
            })
        });
    }

    current_handler(request)
}

fn create_error_response(error: IoError) -> Response<Body> {
    let mut response = Response::new(Body::from(error.to_string()));
    *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("text/plain"),
    );
    response
}
