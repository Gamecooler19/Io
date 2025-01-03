use crate::error::IoError as RuntimeError;
use crate::Result;
use futures::{
    pin_mut,
    task::{Context, Poll},
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{
    future::Future,
    pin::Pin,
    sync::{mpsc, Arc, Mutex},
    task::{self, noop_waker, Wake, Waker},
    thread,
};
use tokio::sync::oneshot;

pub struct Runtime {
    scheduler: Scheduler,
    thread_pool: ThreadPool,
}

impl Runtime {
    pub fn new(threads: usize) -> Self {
        Self {
            scheduler: Scheduler::new(),
            thread_pool: ThreadPool::new(threads),
        }
    }

    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let (handle, task) = Task::new(future);
        self.scheduler.schedule(task);
        handle
    }

    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        let _guard = self.enter();
        let waker = task::noop_waker();
        let mut cx = Context::from_waker(&waker);

        pin_mut!(future);
        loop {
            match future.as_mut().poll(&mut cx) {
                Poll::Ready(output) => return output,
                Poll::Pending => self.scheduler.run_one(),
            }
        }
    }

    pub fn initialize(&self) {
        // Initialize thread pool
        for _ in 0..self.thread_pool.size() {
            let scheduler = self.scheduler.clone();
            self.thread_pool.spawn(move || loop {
                scheduler.run_one();
                std::thread::yield_now();
            });
        }

        // Initialize built-in functions
        let mut context = ExecutionContext::new();
        context.register_builtin_functions();
    }

    pub fn run_program(&self, source: &str) -> Result<Value> {
        let mut context = ExecutionContext::new();
        self.execute_program(source, &mut context)
    }

    fn execute_program(&self, source: &str, context: &mut ExecutionContext) -> Result<Value> {
        // Basic command parsing and execution
        let commands: Vec<&str> = source
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect();

        let mut last_value = Value::Void;

        for cmd in commands {
            last_value = self.execute_command(cmd, context)?;
        }

        Ok(last_value)
    }

    fn execute_command(&self, cmd: &str, context: &mut ExecutionContext) -> Result<Value> {
        if cmd.starts_with("println") {
            let content = self.parse_print_args(cmd)?;
            println!("{}", content);
            Ok(Value::Void)
        } else if cmd.starts_with("let") {
            self.handle_variable_assignment(cmd, context)
        } else if cmd.starts_with("if") {
            self.handle_conditional(cmd, context)
        } else {
            self.evaluate_expression(cmd, context)
        }
    }

    fn parse_print_args(&self, cmd: &str) -> Result<String> {
        let content = cmd
            .trim_start_matches("println")
            .trim_start_matches('(')
            .trim_end_matches(')')
            .trim();

        Ok(content.trim_matches('"').to_string())
    }

    fn handle_variable_assignment(
        &self,
        cmd: &str,
        context: &mut ExecutionContext,
    ) -> Result<Value> {
        let parts: Vec<&str> = cmd.splitn(2, '=').collect();
        if parts.len() != 2 {
            return Err(RuntimeError::SyntaxError("Invalid assignment".into()));
        }

        let var_name = parts[0].trim_start_matches("let").trim().to_string();
        let value = self.evaluate_expression(parts[1].trim(), context)?;

        context.variables.insert(var_name, value.clone());
        Ok(value)
    }

    fn evaluate_expression(&self, expr: &str, context: &ExecutionContext) -> Result<Value> {
        if expr.starts_with("read_line") {
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            Ok(Value::String(input.trim().to_string()))
        } else if let Ok(num) = expr.parse::<f64>() {
            Ok(Value::Number(num))
        } else if expr.starts_with('"') && expr.ends_with('"') {
            Ok(Value::String(expr.trim_matches('"').to_string()))
        } else if let Some(value) = context.variables.get(expr) {
            Ok(value.clone())
        } else {
            self.evaluate_operation(expr, context)
        }
    }

    fn evaluate_operation(&self, expr: &str, context: &ExecutionContext) -> Result<Value> {
        let tokens: Vec<&str> = expr.split_whitespace().collect();
        if tokens.len() != 3 {
            return Err(RuntimeError::SyntaxError("Invalid operation".into()));
        }

        let left = self.evaluate_expression(tokens[0], context)?;
        let right = self.evaluate_expression(tokens[2], context)?;

        match tokens[1] {
            "+" => self.add_values(left, right),
            "-" => self.subtract_values(left, right),
            "*" => self.multiply_values(left, right),
            "/" => self.divide_values(left, right),
            _ => Err(RuntimeError::SyntaxError("Unknown operator".into())),
        }
    }

    fn add_values(&self, left: Value, right: Value) -> Result<Value> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a + b)),
            (Value::String(a), Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
            _ => Err(RuntimeError::TypeError("Invalid types for addition".into())),
        }
    }

    fn subtract_values(&self, left: Value, right: Value) -> Result<Value> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a - b)),
            _ => Err(RuntimeError::TypeError(
                "Invalid types for subtraction".into(),
            )),
        }
    }

    fn multiply_values(&self, left: Value, right: Value) -> Result<Value> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a * b)),
            _ => Err(RuntimeError::TypeError(
                "Invalid types for multiplication".into(),
            )),
        }
    }

    fn divide_values(&self, left: Value, right: Value) -> Result<Value> {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => {
                if b == 0.0 {
                    Err(RuntimeError::RuntimeError("Division by zero".into()))
                } else {
                    Ok(Value::Number(a / b))
                }
            }
            _ => Err(RuntimeError::TypeError("Invalid types for division".into())),
        }
    }

    fn handle_conditional(&self, cmd: &str, context: &mut ExecutionContext) -> Result<Value> {
        let condition_start = cmd.find("if").unwrap() + 2;
        let condition_end = cmd.find("{").unwrap_or_else(|| cmd.len());
        let condition = cmd[condition_start..condition_end].trim();

        let condition_value = self.evaluate_expression(condition, context)?;
        match condition_value {
            Value::Number(n) => Ok(Value::Number(n)),
            Value::Boolean(b) => Ok(Value::Boolean(b)),
            _ => Err(RuntimeError::TypeError(
                "Condition must evaluate to a boolean".into(),
            )),
        }
    }

    fn handle_loop(&self, cmd: &str, context: &mut ExecutionContext) -> Result<Value> {
        let loop_type = if cmd.starts_with("while") {
            "while"
        } else {
            "for"
        };
        let body_start = cmd
            .find("{")
            .ok_or_else(|| RuntimeError::SyntaxError("Missing loop body".into()))?;
        let body = &cmd[body_start + 1..cmd.len() - 1];

        match loop_type {
            "while" => self.execute_while_loop(&cmd[5..body_start].trim(), body, context),
            "for" => self.execute_for_loop(&cmd[3..body_start].trim(), body, context),
            _ => unreachable!(),
        }
    }

    fn execute_while_loop(
        &self,
        condition: &str,
        body: &str,
        context: &mut ExecutionContext,
    ) -> Result<Value> {
        while self.evaluate_expression(condition, context)?.is_truthy() {
            self.execute_block(body, context)?;
        }
        Ok(Value::Void)
    }

    fn execute_for_loop(
        &self,
        setup: &str,
        body: &str,
        context: &mut ExecutionContext,
    ) -> Result<Value> {
        let parts: Vec<&str> = setup.split(";").collect();
        if parts.len() != 3 {
            return Err(RuntimeError::SyntaxError("Invalid for loop syntax".into()));
        }

        // Initialize
        self.execute_command(parts[0], context)?;

        // Condition and iteration
        while self.evaluate_expression(parts[1], context)?.is_truthy() {
            self.execute_block(body, context)?;
            self.execute_command(parts[2], context)?;
        }

        Ok(Value::Void)
    }

    fn execute_block(&self, block: &str, context: &mut ExecutionContext) -> Result<Value> {
        let mut last_value = Value::Void;
        for cmd in block.split(";").map(str::trim).filter(|s| !s.is_empty()) {
            last_value = self.execute_command(cmd, context)?;
        }
        Ok(last_value)
    }

    fn enter(&self) -> RuntimeGuard {
        RUNTIME_METRICS.with(|metrics| {
            metrics.borrow_mut().start_time = std::time::Instant::now();
        });

        RuntimeGuard { _private: () }
    }

    fn some_runtime_function(&self) -> Result<()> {
        // ...existing code...
        Err(error::IoError::type_error("Runtime error message")) // Specify module
                                                                 // ...existing code...
    }
}

struct RuntimeGuard {
    _private: (),
}

impl Drop for RuntimeGuard {
    fn drop(&mut self) {
        RUNTIME_METRICS.with(|metrics| {
            let mut metrics = metrics.borrow_mut();
            metrics.total_runtime += metrics.start_time.elapsed();
        });
    }
}

struct Scheduler {
    tasks: Arc<Mutex<Vec<Arc<Task>>>>,
}

impl Scheduler {
    fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn schedule(&self, task: Arc<Task>) {
        self.tasks.lock().unwrap().push(task);
    }

    fn run_one(&self) {
        if let Some(task) = self.tasks.lock().unwrap().pop() {
            task.poll();
        }
    }
}

struct Task {
    future: Mutex<Pin<Box<dyn Future<Output = ()> + Send>>>,
    waker: Waker,
}

impl Task {
    fn new<F>(future: F) -> (JoinHandle<F::Output>, Arc<Self>)
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let (sender, receiver) = futures::channel::oneshot::channel();
        let future = Box::pin(async move {
            let result = future.await;
            let _ = sender.send(result);
        });

        let task = Arc::new(Task {
            future: Mutex::new(future),
            waker: Arc::new(TaskWaker).into(),
        });

        let handle = JoinHandle {
            task: task.clone(),
            receiver,
        };
        (handle, task)
    }

    fn poll(self: &Arc<Self>) {
        let mut future = self.future.lock().unwrap();
        let waker = &self.waker;
        let mut cx = Context::from_waker(waker);
        let _ = future.as_mut().poll(&mut cx);
    }
}

struct TaskWaker;

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        let scheduler = GLOBAL_SCHEDULER.get().expect("Runtime not initialized");
        let waker_ref = Arc::new(self);
        let waker = Waker::from(waker_ref);

        // Create new task context
        let mut context = Context::from_waker(&waker);

        // Schedule the task for execution
        scheduler.schedule(Arc::new(Task {
            future: Mutex::new(Box::pin(async {
                // Task execution logic
                let mut future = self.future.lock().unwrap();
                if let Poll::Ready(()) = future.as_mut().poll(&mut context) {
                    // Task completed
                    drop(future);
                }
            })),
            waker: waker.clone(),
        }));
    }
}

// Add comprehensive thread-local storage
thread_local! {
    static GLOBAL_SCHEDULER: RefCell<Option<Arc<Scheduler>>> = RefCell::new(None);
    static TASK_LOCAL_STORAGE: RefCell<HashMap<String, Value>> = RefCell::new(HashMap::new());
    static ERROR_CONTEXT: RefCell<Vec<ErrorContext>> = RefCell::new(Vec::new());
    static RUNTIME_METRICS: RefCell<RuntimeMetrics> = RefCell::new(RuntimeMetrics::new());
    static THREAD_ID: RefCell<usize> = RefCell::new(0);
    static MEMORY_PROFILE: RefCell<MemoryProfile> = RefCell::new(MemoryProfile::new());
    static CALL_STACK: RefCell<Vec<CallFrame>> = RefCell::new(Vec::new());
    static DEBUG_LOG: RefCell<Vec<DebugEntry>> = RefCell::new(Vec::new());
}

struct MemoryProfile {
    allocations: HashMap<String, usize>,
    deallocations: HashMap<String, usize>,
    current_usage: usize,
    peak_usage: usize,
    allocation_sites: HashMap<String, Vec<(&'static str, usize, usize)>>,
}

impl MemoryProfile {
    fn new() -> Self {
        Self {
            allocations: HashMap::new(),
            deallocations: HashMap::new(),
            current_usage: 0,
            peak_usage: 0,
            allocation_sites: HashMap::new(),
        }
    }

    fn record_allocation(&mut self, size: usize, type_name: &str, file: &'static str, line: usize) {
        self.current_usage += size;
        self.peak_usage = self.peak_usage.max(self.current_usage);

        *self.allocations.entry(type_name.to_string()).or_insert(0) += size;
        self.allocation_sites
            .entry(type_name.to_string())
            .or_default()
            .push((file, line, size));
    }

    fn record_deallocation(&mut self, size: usize, type_name: &str) {
        self.current_usage = self.current_usage.saturating_sub(size);
        *self.deallocations.entry(type_name.to_string()).or_insert(0) += size;
    }
}

struct CallFrame {
    function: String,
    file: String,
    line: usize,
    column: usize,
    locals: HashMap<String, Value>,
    start_time: std::time::Instant,
}

struct DebugEntry {
    timestamp: std::time::SystemTime,
    level: DebugLevel,
    message: String,
    context: HashMap<String, String>,
}

#[derive(Clone, Copy, Debug)]
enum DebugLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

struct RuntimeMetrics {
    tasks_completed: AtomicUsize,
    tasks_failed: AtomicUsize,
    total_runtime: std::time::Duration,
    peak_memory_usage: AtomicUsize,
    error_count: HashMap<ErrorCategory, AtomicUsize>,
    start_time: std::time::Instant,
    memory_usage: Vec<(std::time::Instant, usize)>,
    execution_times: HashMap<String, std::time::Duration>,
    last_gc_run: std::time::Instant,
}

impl RuntimeMetrics {
    fn new() -> Self {
        Self {
            tasks_completed: AtomicUsize::new(0),
            tasks_failed: AtomicUsize::new(0),
            total_runtime: std::time::Duration::default(),
            peak_memory_usage: AtomicUsize::new(0),
            error_count: ErrorCategory::all()
                .into_iter()
                .map(|cat| (cat, AtomicUsize::new(0)))
                .collect(),
            start_time: std::time::Instant::now(),
            memory_usage: Vec::new(),
            execution_times: HashMap::new(),
            last_gc_run: std::time::Instant::now(),
        }
    }

    fn record_error(&self, error: &RuntimeError) {
        if let Some(counter) = self.error_count.get(&error.category()) {
            counter.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn track_memory_usage(&mut self, usage: usize) {
        self.memory_usage.push((std::time::Instant::now(), usage));
        if usage > self.peak_memory_usage.load(Ordering::Relaxed) {
            self.peak_memory_usage.store(usage, Ordering::Relaxed);
        }
    }

    fn record_execution_time(&mut self, operation: &str, duration: std::time::Duration) {
        self.execution_times
            .entry(operation.to_string())
            .and_modify(|t| *t += duration)
            .or_insert(duration);
    }

    fn should_trigger_gc(&self) -> bool {
        self.peak_memory_usage.load(Ordering::Relaxed) > 1024 * 1024 * 100 // 100MB
            || self.last_gc_run.elapsed() > std::time::Duration::from_secs(300) // 5 minutes
    }
}

#[derive(Debug)]
struct ErrorContext {
    file: String,
    line: usize,
    column: usize,
    function: String,
    variables: HashMap<String, Value>,
}

impl From<&ErrorContext> for StackFrame {
    fn from(context: &ErrorContext) -> Self {
        StackFrame {
            file: context.file.clone(),
            line: context.line,
            column: context.column,
            function: context.function.clone(),
        }
    }
}

#[derive(Debug)]
pub struct StackFrame {
    file: String,
    line: usize,
    column: usize,
    function: String,
}

pub struct JoinHandle<T> {
    task: Arc<Task>,
    receiver: futures::channel::oneshot::Receiver<T>,
}

impl<T> Future for JoinHandle<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.receiver).poll(cx)
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Void,
    BuiltinFunction(Box<dyn Fn(Vec<Value>) -> Result<Value> + Send + Sync>),
    Boolean(bool),
    Object(HashMap<String, Value>),
}

impl Value {
    fn is_truthy(&self) -> bool {
        match self {
            Value::Number(n) => *n != 0.0,
            Value::Boolean(b) => *b,
            Value::String(s) => !s.is_empty(),
            Value::Array(arr) => !arr.is_empty(),
            Value::Void => false,
            Value::BuiltinFunction(_) => true,
            Value::Object(obj) => !obj.is_empty(),
        }
    }

    fn to_string(&self) -> String {
        match self {
            Value::Number(n) => n.to_string(),
            Value::Boolean(b) => b.to_string(),
            Value::String(s) => s.clone(),
            Value::Array(arr) => format!(
                "[{}]",
                arr.iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Value::Void => "void".to_string(),
            Value::BuiltinFunction(_) => "<function>".to_string(),
            Value::Object(obj) => format!("{:?}", obj),
        }
    }
}

struct ExecutionContext {
    variables: HashMap<String, Value>,
}

impl ExecutionContext {
    fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    fn register_builtin_functions(&mut self) {
        // Register built-in functions
        self.variables.insert(
            "to_string".to_string(),
            Value::BuiltinFunction(Box::new(|args| {
                if let Some(arg) = args.get(0) {
                    Ok(Value::String(format!("{:?}", arg)))
                } else {
                    Err(RuntimeError::RuntimeError(
                        "to_string requires one argument".into(),
                    ))
                }
            })),
        );

        self.variables.insert(
            "parse_int".to_string(),
            Value::BuiltinFunction(Box::new(|args| {
                if let Some(Value::String(s)) = args.get(0) {
                    match s.parse::<f64>() {
                        Ok(n) => Ok(Value::Number(n)),
                        Err(_) => Err(RuntimeError::RuntimeError("Failed to parse integer".into())),
                    }
                } else {
                    Err(RuntimeError::TypeError(
                        "parse_int requires a string argument".into(),
                    ))
                }
            })),
        );

        self.variables.insert(
            "len".to_string(),
            Value::BuiltinFunction(Box::new(|args| {
                if let Some(Value::Array(arr)) = args.get(0) {
                    Ok(Value::Number(arr.len() as f64))
                } else {
                    Err(RuntimeError::TypeError(
                        "len requires an array argument".into(),
                    ))
                }
            })),
        );
    }
}

// Add comprehensive error types
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum ErrorCategory {
    Syntax,
    Type,
    Runtime,
    Concurrency,
    System,
    Memory,
    Module,
    Library,
}

impl ErrorCategory {
    fn all() -> Vec<ErrorCategory> {
        vec![
            ErrorCategory::Syntax,
            ErrorCategory::Type,
            ErrorCategory::Runtime,
            ErrorCategory::Concurrency,
            ErrorCategory::System,
            ErrorCategory::Memory,
            ErrorCategory::Module,
            ErrorCategory::Library,
        ]
    }
}

// Extend IoError implementation
impl RuntimeError {
    // ...existing error variants...

    // Add more specific error variants
    pub fn validation_error(message: impl Into<String>) -> Self {
        RuntimeError::ValidationError {
            message: message.into(),
        }
    }

    pub fn concurrency_error(message: impl Into<String>) -> Self {
        RuntimeError::ConcurrencyError {
            message: message.into(),
        }
    }

    pub fn resource_error(resource: impl Into<String>, operation: impl Into<String>) -> Self {
        RuntimeError::ResourceError {
            resource: resource.into(),
            operation: operation.into(),
        }
    }

    pub fn security_error(message: impl Into<String>, severity: SecurityLevel) -> Self {
        RuntimeError::SecurityError {
            message: message.into(),
            severity,
        }
    }

    fn category(&self) -> ErrorCategory {
        match self {
            RuntimeError::SyntaxError(_) | RuntimeError::ParseError { .. } => ErrorCategory::Syntax,
            RuntimeError::TypeError(_) | RuntimeError::TypeMismatch { .. } => ErrorCategory::Type,
            RuntimeError::RuntimeError(_) | RuntimeError::DivisionByZero => ErrorCategory::Runtime,
            RuntimeError::DeadLock(_) | RuntimeError::TaskPanic(_) => ErrorCategory::Concurrency,
            RuntimeError::OutOfMemory | RuntimeError::BufferOverflow => ErrorCategory::Memory,
            RuntimeError::ModuleNotFound(_) | RuntimeError::CircularDependency(_) => {
                ErrorCategory::Module
            }
            RuntimeError::ValueError(_) | RuntimeError::ConversionError { .. } => {
                ErrorCategory::Library
            }
            _ => ErrorCategory::System,
        }
    }

    pub fn backtrace(&self) -> Option<Vec<StackFrame>> {
        ERROR_CONTEXT.with(|ctx| {
            let ctx = ctx.borrow();
            if (!ctx.is_empty()) {
                Some(ctx.iter().map(|c| c.into()).collect())
            } else {
                None
            }
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SecurityLevel {
    Low,
    Medium,
    High,
    Critical,
}

// Add new error variants
#[derive(Debug)]
pub enum IoError {
    // ...existing variants...
    ValidationError {
        message: String,
    },
    ConcurrencyError {
        message: String,
    },
    ResourceError {
        resource: String,
        operation: String,
    },
    SecurityError {
        message: String,
        severity: SecurityLevel,
    },
    NetworkError {
        endpoint: String,
        error_code: u16,
        message: String,
    },
    DatabaseError {
        operation: String,
        table: String,
        message: String,
    },
    ConfigError {
        key: String,
        message: String,
    },
    SerializationError {
        target_type: String,
        message: String,
    },
    StackOverflow,
    NullPointer(String),
    DeadLock(String),
    TaskPanic(String),
    ChannelError(String),
    FileNotFound(String),
    PermissionDenied {
        path: String,
        operation: String,
    },
    OutOfMemory,
    InvalidPointer(String),
    BufferOverflow,
    ModuleNotFound(String),
    CircularDependency(Vec<String>),
    ExportError(String),
    ValueError(String),
    ConversionError {
        from: String,
        to: String,
        value: String,
    },
}

// Add format implementation for new variants
impl std::fmt::Display for IoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // ...existing variants...
            RuntimeError::ValidationError { message } => {
                write!(f, "Validation error: {}", message)
            }
            RuntimeError::ConcurrencyError { message } => {
                write!(f, "Concurrency error: {}", message)
            }
            RuntimeError::ResourceError {
                resource,
                operation,
            } => {
                write!(f, "Resource error during {} on {}", operation, resource)
            }
            RuntimeError::SecurityError { message, severity } => {
                write!(f, "{:?} security violation: {}", severity, message)
            }
            RuntimeError::NetworkError {
                endpoint,
                error_code,
                message,
            } => {
                write!(
                    f,
                    "Network error {} at {}: {}",
                    error_code, endpoint, message
                )
            }
            RuntimeError::DatabaseError {
                operation,
                table,
                message,
            } => {
                write!(
                    f,
                    "Database error during {} on table {}: {}",
                    operation, table, message
                )
            }
            RuntimeError::ConfigError { key, message } => {
                write!(f, "Configuration error for {}: {}", key, message)
            }
            RuntimeError::SerializationError {
                target_type,
                message,
            } => {
                write!(
                    f,
                    "Serialization error for type {}: {}",
                    target_type, message
                )
            }
        }
    }
}

// Add helper methods for error creation
impl RuntimeError {
    pub fn syntax_error(message: impl Into<String>) -> Self {
        RuntimeError::SyntaxError(message.into())
    }

    pub fn type_error(message: impl Into<String>) -> Self {
        RuntimeError::TypeError(message.into())
    }

    pub fn runtime_error(message: impl Into<String>) -> Self {
        RuntimeError::RuntimeError(message.into())
    }

    pub fn module_error(message: impl Into<String>) -> Self {
        RuntimeError::ModuleNotFound(message.into())
    }

    pub fn is_recoverable(&self) -> bool {
        !matches!(
            self,
            RuntimeError::StackOverflow | RuntimeError::OutOfMemory | RuntimeError::DeadLock(_)
        )
    }
}

// Add thread pool implementation
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
}

impl ThreadPool {
    pub fn new(size: usize) -> Self {
        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);
        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        Self { workers, sender }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.sender.send(Message::NewJob(job)).unwrap();
    }

    pub fn size(&self) -> usize {
        self.workers.len()
    }
}

enum Message {
    NewJob(Job),
    Terminate,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Self {
        let thread = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv().unwrap();
            match message {
                Message::NewJob(job) => {
                    job();
                }
                Message::Terminate => {
                    break;
                }
            }
        });

        Self {
            id,
            thread: Some(thread),
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        for _ in &self.workers {
            self.sender.send(Message::Terminate).unwrap();
        }

        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

// Helper methods for thread-local access
pub fn with_task_local<F, R>(key: &str, f: F) -> R
where
    F: FnOnce(Option<&Value>) -> R,
{
    TASK_LOCAL_STORAGE.with(|storage| f(storage.borrow().get(key)))
}

pub fn set_task_local(key: String, value: Value) {
    TASK_LOCAL_STORAGE.with(|storage| {
        storage.borrow_mut().insert(key, value);
    });
}

pub fn push_call_frame(frame: CallFrame) {
    CALL_STACK.with(|stack| {
        stack.borrow_mut().push(frame);
    });
}

pub fn pop_call_frame() -> Option<CallFrame> {
    CALL_STACK.with(|stack| stack.borrow_mut().pop())
}

pub fn log_debug(level: DebugLevel, message: String, context: HashMap<String, String>) {
    DEBUG_LOG.with(|log| {
        log.borrow_mut().push(DebugEntry {
            timestamp: std::time::SystemTime::now(),
            level,
            message,
            context,
        });
    });
}
