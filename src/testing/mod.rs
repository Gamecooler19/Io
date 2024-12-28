pub mod benchmark;
pub mod assertions;

use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use parking_lot::RwLock;
use crate::{Result, error::IoError};

#[derive(Debug)]
pub struct TestSuite {
    name: String,
    tests: Vec<TestCase>,
    setup: Option<Arc<dyn Fn() -> Result<()> + Send + Sync>>,
    teardown: Option<Arc<dyn Fn() -> Result<()> + Send + Sync>>,
    metrics: Arc<TestMetrics>,
}

#[derive(Debug)]
pub struct TestCase {
    name: String,
    function: Arc<dyn Fn() -> Result<()> + Send + Sync>,
    expected_duration: Option<Duration>,
    timeout: Option<Duration>,
    should_panic: bool,
}

impl TestSuite {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            tests: Vec::new(),
            setup: None,
            teardown: None,
            metrics: Arc::new(TestMetrics::new()),
        }
    }

    pub fn add_test<F>(&mut self, name: &str, test_fn: F) -> &mut TestCase
    where
        F: Fn() -> Result<()> + Send + Sync + 'static,
    {
        let test = TestCase {
            name: name.to_string(),
            function: Arc::new(test_fn),
            expected_duration: None,
            timeout: Some(Duration::from_secs(30)), // Default timeout
            should_panic: false,
        };
        self.tests.push(test);
        self.tests.last_mut().unwrap()
    }

    pub fn run(&self) -> Result<TestResults> {
        let start_time = Instant::now();
        let mut results = TestResults::new();

        println!("\nRunning test suite: {}", self.name);

        for test in &self.tests {
            // Run setup if defined
            if let Some(setup) = &self.setup {
                if let Err(e) = setup() {
                    results.add_failure(&test.name, TestFailure::SetupError(e));
                    continue;
                }
            }

            // Run the test with timeout
            let test_result = self.run_test_with_timeout(test);
            match test_result {
                Ok(duration) => {
                    if test.should_panic {
                        results.add_failure(&test.name, TestFailure::ExpectedPanic);
                    } else {
                        results.add_success(&test.name, duration);
                    }
                }
                Err(e) => {
                    if test.should_panic {
                        results.add_success(&test.name, Duration::from_secs(0));
                    } else {
                        results.add_failure(&test.name, TestFailure::Error(e));
                    }
                }
            }

            // Run teardown if defined
            if let Some(teardown) = &self.teardown {
                if let Err(e) = teardown() {
                    results.add_failure(&test.name, TestFailure::TeardownError(e));
                }
            }
        }

        results.total_duration = start_time.elapsed();
        Ok(results)
    }

    fn run_test_with_timeout(&self, test: &TestCase) -> Result<Duration> {
        let start = Instant::now();
        let timeout = test.timeout.unwrap_or(Duration::from_secs(30));

        let result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async {
                let test_future = (test.function)();
                tokio::time::timeout(timeout, async { test_future }).await
            });

        match result {
            Ok(Ok(_)) => {
                let duration = start.elapsed();
                if let Some(expected) = test.expected_duration {
                    if duration > expected {
                        return Err(IoError::runtime_error(
                            format!("Test took {:?}, expected {:?}", duration, expected)
                        ));
                    }
                }
                Ok(duration)
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(IoError::runtime_error("Test timed out")),
        }
    }
}

#[derive(Debug)]
pub enum TestFailure {
    Error(IoError),
    Timeout,
    ExpectedPanic,
    SetupError(IoError),
    TeardownError(IoError),
}

#[derive(Debug, Default)]
pub struct TestResults {
    successes: Vec<(String, Duration)>,
    failures: Vec<(String, TestFailure)>,
    total_duration: Duration,
}

impl TestResults {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_success(&mut self, name: &str, duration: Duration) {
        self.successes.push((name.to_string(), duration));
    }

    pub fn add_failure(&mut self, name: &str, failure: TestFailure) {
        self.failures.push((name.to_string(), failure));
    }

    pub fn print_summary(&self) {
        println!("\nTest Summary:");
        println!("-------------");
        println!("Total tests: {}", self.successes.len() + self.failures.len());
        println!("Successes: {}", self.successes.len());
        println!("Failures: {}", self.failures.len());
        println!("Total duration: {:?}", self.total_duration);

        if !self.failures.is_empty() {
            println!("\nFailures:");
            for (name, failure) in &self.failures {
                println!("  {} - {:?}", name, failure);
            }
        }
    }
}
