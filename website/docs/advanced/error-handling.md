# Error Handling in Io

## Error Types Hierarchy
```rust
/// Base error type for all Io operations
#[derive(Debug)]
pub enum IoError {
    /// File system related errors
    Filesystem {
        path: PathBuf,
        kind: FsErrorKind,
    },
    /// Network related errors
    Network {
        url: String,
        kind: NetworkErrorKind,
    },
    /// Database related errors
    Database {
        query: String,
        kind: DbErrorKind,
    },
    /// Application logic errors
    Application {
        context: String,
        details: Box<dyn Error>,
    },
}

#[derive(Debug)]
pub enum FsErrorKind {
    NotFound,
    PermissionDenied,
    AlreadyExists,
    Other(Box<dyn Error>),
}

#[derive(Debug)]
pub enum NetworkErrorKind {
    ConnectionRefused,
    Timeout,
    InvalidAddress,
    Other(Box<dyn Error>),
}

#[derive(Debug)]
pub enum DbErrorKind {
    ConnectionFailed,
    QueryError,
    DeadlockDetected,
    Other(Box<dyn Error>),
}

/// Custom error implementation with context
impl Error for IoError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            IoError::Application { details, .. } => Some(details.as_ref()),
            _ => None,
        }
    }
}

impl std::fmt::Display for IoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IoError::Filesystem { path, kind } => {
                write!(f, "Filesystem error at {}: {:?}", path.display(), kind)
            }
            IoError::Network { url, kind } => {
                write!(f, "Network error for {}: {:?}", url, kind)
            }
            IoError::Database { query, kind } => {
                write!(f, "Database error in query '{}': {:?}", query, kind)
            }
            IoError::Application { context, details } => {
                write!(f, "Application error in {}: {}", context, details)
            }
        }
    }
}
```

## Error Handling Patterns

### Result Combinators
```rust
fn process_data() -> Result<ProcessedData, IoError> {
    read_file("input.txt")
        .map_err(|e| IoError::Filesystem { 
            path: "input.txt".into(), 
            kind: e.into() 
        })?
        .parse()
        .map_err(|e| IoError::Application {
            context: "parsing".into(),
            details: Box::new(e),
        })?
}
```

### Error Context
```rust
#[derive(Debug)]
struct ErrorContext {
    timestamp: DateTime<Utc>,
    thread_id: ThreadId,
    trace_id: UUID,
    user_id: Option<String>,
}

impl ErrorContext {
    fn capture() -> Self {
        Self {
            timestamp: Utc::now(),
            thread_id: std::thread::current().id(),
            trace_id: Uuid::new_v4(),
            user_id: None,
        }
    }

    fn with_user(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }
}
```

## Structured Logging Integration

### Log Levels and Filtering
```rust
#[macro_export]
macro_rules! log_error {
    ($error:expr, $context:expr) => {
        logger::error!({
            error: $error,
            context: $context,
            location: file!(),
            line: line!(),
            timestamp: chrono::Utc::now(),
        });
    }
}
```

## Recovery Strategies

### Circuit Breaker Pattern
```rust
pub struct CircuitBreaker<T> {
    operation: Box<dyn Fn() -> Result<T, IoError>>,
    failure_threshold: u32,
    reset_timeout: Duration,
    state: CircuitState,
}

#[derive(Debug, Clone, Copy)]
enum CircuitState {
    Closed,
    Open(Instant),
    HalfOpen,
}

impl<T> CircuitBreaker<T> {
    pub fn new(
        operation: impl Fn() -> Result<T, IoError> + 'static,
        failure_threshold: u32,
        reset_timeout: Duration,
    ) -> Self {
        Self {
            operation: Box::new(operation),
            failure_threshold,
            reset_timeout,
            state: CircuitState::Closed,
        }
    }

    pub async fn execute(&mut self) -> Result<T, IoError> {
        match self.state {
            CircuitState::Closed => self.try_operation(),
            CircuitState::Open(ref last_failure) => {
                if self.should_reset(last_failure) {
                    self.state = CircuitState::HalfOpen;
                    self.try_operation()
                } else {
                    Err(IoError::Circuit("Circuit is open".into()))
                }
            }
            CircuitState::HalfOpen => {
                // Implementation for half-open state
            }
        }
    }

    fn should_reset(&self, last_failure: &Instant) -> bool {
        Instant::now().duration_since(*last_failure) >= self.reset_timeout
    }

    fn try_operation(&mut self) -> Result<T, IoError> {
        match (self.operation)() {
            Ok(result) => {
                self
```

## Production Monitoring

### Error Metrics
```rust
pub struct ErrorMetrics {
    error_count: AtomicUsize,
    error_types: ConcurrentHashMap<ErrorType, AtomicUsize>,
    last_error: AtomicCell<Option<ErrorContext>>,
}

impl ErrorMetrics {
    pub fn record_error(&self, error: &IoError) {
        // Implementation for recording error metrics
    }
}
```

## Best Practices

### Error Propagation
1. Always add context when propagating errors
2. Use structured logging for all errors
3. Implement proper error recovery mechanisms
4. Monitor error rates and patterns
5. Set up alerts for critical errors

### Error Documentation
```rust
/// Processes a transaction with proper error handling
///
/// # Errors
/// This function will return an error in the following situations:
/// - Database connection failure (`IoError::Database`)
/// - Invalid transaction data (`IoError::Application`)
/// - Network timeout during processing (`IoError::Network`)
///
/// # Recovery
/// - Database errors: Automatic retry up to 3 times
/// - Network errors: Circuit breaker pattern
pub async fn process_transaction(
    transaction: Transaction
) -> Result<Receipt, IoError> {
    // Implementation
}
```