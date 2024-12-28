use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use parking_lot::RwLock;
use tokio::time::sleep;

#[derive(Debug, Clone)]
pub struct RateLimit {
    requests: usize,
    window: Duration,
}

#[derive(Debug)]
struct RateLimitCounter {
    count: usize,
    window_start: Instant,
}

pub struct RateLimiter {
    limits: HashMap<String, RateLimit>,
    counters: Arc<RwLock<HashMap<(String, String), RateLimitCounter>>>, // (endpoint, client_id) -> counter
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            limits: HashMap::new(),
            counters: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn add_limit(&mut self, endpoint: &str, requests: usize, window: Duration) {
        self.limits.insert(
            endpoint.to_string(),
            RateLimit { requests, window },
        );
    }

    pub async fn check_rate_limit(&self, endpoint: &str, client_id: &str) -> Result<(), IoError> {
        let limit = self.limits.get(endpoint).ok_or_else(|| {
            IoError::validation_error(format!("No rate limit defined for endpoint: {}", endpoint))
        })?;

        let key = (endpoint.to_string(), client_id.to_string());
        let now = Instant::now();

        let mut counters = self.counters.write();
        let counter = counters.entry(key.clone()).or_insert_with(|| RateLimitCounter {
            count: 0,
            window_start: now,
        });

        // Reset counter if window has passed
        if now.duration_since(counter.window_start) >= limit.window {
            counter.count = 0;
            counter.window_start = now;
        }

        if counter.count >= limit.requests {
            let wait_time = limit.window - now.duration_since(counter.window_start);
            drop(counters); // Release lock before sleeping
            sleep(wait_time).await;
            return Err(IoError::runtime_error("Rate limit exceeded"));
        }

        counter.count += 1;
        Ok(())
    }
}

pub struct RateLimitMiddleware<'ctx> {
    context: &'ctx inkwell::context::Context,
    rate_limiter: Arc<RateLimiter>,
}

impl<'ctx> RateLimitMiddleware<'ctx> {
    pub fn new(context: &'ctx inkwell::context::Context, rate_limiter: Arc<RateLimiter>) -> Self {
        Self {
            context,
            rate_limiter,
        }
    }

    pub fn generate_middleware(
        &self,
        codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>,
    ) -> Result<inkwell::values::FunctionValue<'ctx>> {
        let fn_type = codegen.get_type("HttpResponse")?.fn_type(
            &[
                codegen.get_type("HttpRequest")?.into(),
                codegen.get_type("Context")?.into(),
            ],
            false,
        );

        let function = codegen.module.add_function("rate_limit_middleware", fn_type, None);
        let builder = codegen.context.create_builder();
        let entry = codegen.context.append_basic_block(function, "entry");
        builder.position_at_end(entry);

        // Generate rate limit check logic
        self.generate_rate_limit_check(&builder, function)?;

        Ok(function)
    }
}
