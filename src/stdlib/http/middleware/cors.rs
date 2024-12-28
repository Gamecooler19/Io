use hyper::{Body, Request, Response, header};
use crate::{Result, error::IoError};
use super::Middleware;

pub struct CorsMiddleware {
    allow_origin: String,
    allow_methods: Vec<String>,
    allow_headers: Vec<String>,
    allow_credentials: bool,
    max_age: u32,
}

impl CorsMiddleware {
    pub fn new(config: CorsConfig) -> Self {
        Self {
            allow_origin: config.allow_origin,
            allow_methods: config.allow_methods,
            allow_headers: config.allow_headers,
            allow_credentials: config.allow_credentials,
            max_age: config.max_age,
        }
    }
}

#[async_trait::async_trait]
impl Middleware for CorsMiddleware {
    async fn process(
        &self,
        request: Request<Body>,
        next: Box<dyn Fn(Request<Body>) -> Result<Response<Body>> + Send>,
    ) -> Result<Response<Body>> {
        if request.method() == hyper::Method::OPTIONS {
            // Handle preflight request
            let mut response = Response::new(Body::empty());
            self.add_cors_headers(response.headers_mut());
            Ok(response)
        } else {
            // Handle actual request
            let mut response = next(request)?;
            self.add_cors_headers(response.headers_mut());
            Ok(response)
        }
    }
}

impl CorsMiddleware {
    fn add_cors_headers(&self, headers: &mut header::HeaderMap) {
        headers.insert(
            header::ACCESS_CONTROL_ALLOW_ORIGIN,
            header::HeaderValue::from_str(&self.allow_origin).unwrap(),
        );

        headers.insert(
            header::ACCESS_CONTROL_ALLOW_METHODS,
            header::HeaderValue::from_str(&self.allow_methods.join(", ")).unwrap(),
        );

        headers.insert(
            header::ACCESS_CONTROL_ALLOW_HEADERS,
            header::HeaderValue::from_str(&self.allow_headers.join(", ")).unwrap(),
        );

        if self.allow_credentials {
            headers.insert(
                header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
                header::HeaderValue::from_static("true"),
            );
        }

        headers.insert(
            header::ACCESS_CONTROL_MAX_AGE,
            header::HeaderValue::from_str(&self.max_age.to_string()).unwrap(),
        );
    }
}

pub struct CorsConfig {
    pub allow_origin: String,
    pub allow_methods: Vec<String>,
    pub allow_headers: Vec<String>,
    pub allow_credentials: bool,
    pub max_age: u32,
}
