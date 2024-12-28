use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use parking_lot::RwLock;
use statrs::statistics::Statistics;
use crate::Result;

#[derive(Debug)]
pub struct Benchmark {
    name: String,
    function: Arc<dyn Fn() -> Result<()> + Send + Sync>,
    iterations: usize,
    warmup_iterations: usize,
    metrics: Arc<BenchmarkMetrics>,
}

#[derive(Debug)]
pub struct BenchmarkMetrics {
    samples: RwLock<Vec<Duration>>,
    throughput: RwLock<Option<f64>>,
    memory_usage: RwLock<Vec<usize>>,
}

impl Benchmark {
    pub fn new<F>(name: &str, function: F) -> Self
    where
        F: Fn() -> Result<()> + Send + Sync + 'static,
    {
        Self {
            name: name.to_string(),
            function: Arc::new(function),
            iterations: 1000,
            warmup_iterations: 100,
            metrics: Arc::new(BenchmarkMetrics::new()),
        }
    }

    pub fn with_iterations(mut self, iterations: usize) -> Self {
        self.iterations = iterations;
        self
    }

    pub fn with_warmup(mut self, warmup_iterations: usize) -> Self {
        self.warmup_iterations = warmup_iterations;
        self
    }

    pub fn run(&self) -> Result<BenchmarkResults> {
        println!("\nRunning benchmark: {}", self.name);
        println!("Warming up...");

        // Warmup phase
        for _ in 0..self.warmup_iterations {
            (self.function)()?;
        }

        println!("Running {} iterations...", self.iterations);
        let mut samples = Vec::with_capacity(self.iterations);
        let mut memory_samples = Vec::with_capacity(self.iterations);

        // Main benchmark
        for _ in 0..self.iterations {
            let start = Instant::now();
            let initial_memory = memory_usage();
            
            (self.function)()?;
            
            let duration = start.elapsed();
            let final_memory = memory_usage();
            
            samples.push(duration);
            memory_samples.push(final_memory - initial_memory);
        }

        // Calculate statistics
        let results = BenchmarkResults {
            name: self.name.clone(),
            iterations: self.iterations,
            samples,
            memory_samples,
        };

        println!("Benchmark complete. Results:");
        results.print_summary();
        Ok(results)
    }
}

#[derive(Debug)]
pub struct BenchmarkResults {
    name: String,
    iterations: usize,
    samples: Vec<Duration>,
    memory_samples: Vec<usize>,
}

impl BenchmarkResults {
    pub fn print_summary(&self) {
        let durations: Vec<f64> = self.samples
            .iter()
            .map(|d| d.as_secs_f64() * 1000.0) // Convert to milliseconds
            .collect();

        println!("\nBenchmark: {}", self.name);
        println!("Iterations: {}", self.iterations);
        println!("Mean: {:.3} ms", durations.mean());
        println!("Median: {:.3} ms", durations.median());
        println!("Std Dev: {:.3} ms", durations.std_dev());
        println!("Min: {:.3} ms", durations.min());
        println!("Max: {:.3} ms", durations.max());
        
        let memory_mean = self.memory_samples.iter().sum::<usize>() as f64 
            / self.memory_samples.len() as f64;
        println!("Average Memory Usage: {:.2} KB", memory_mean / 1024.0);
    }
}

fn memory_usage() -> usize {
    // Get current process memory usage
    // This is a simplified version - in production, use proper system APIs
    std::process::Command::new("ps")
        .args(&["ux", &std::process::id().to_string()])
        .output()
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .nth(1)
                .and_then(|line| {
                    line.split_whitespace()
                        .nth(5)
                        .and_then(|mem| mem.parse::<usize>().ok())
                })
                .unwrap_or(0)
        })
        .unwrap_or(0)
}
