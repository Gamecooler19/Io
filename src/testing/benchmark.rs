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

#[cfg(target_os = "linux")]
fn memory_usage() -> usize {
    use std::fs::File;
    use std::io::Read;

    // Try to read from /proc/self/statm
    if let Ok(mut file) = File::open("/proc/self/statm") {
        let mut contents = String::new();
        if file.read_to_string(&mut contents).is_ok() {
            if let Some(rss) = contents
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.parse::<usize>().ok())
            {
                return rss * page_size();
            }
        }
    }

    // Fallback to /proc/self/status
    if let Ok(mut file) = File::open("/proc/self/status") {
        let mut contents = String::new();
        if file.read_to_string(&mut contents).is_ok() {
            if let Some(line) = contents.lines().find(|l| l.starts_with("VmRSS:")) {
                if let Some(kb) = line
                    .split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse::<usize>().ok())
                {
                    return kb * 1024;
                }
            }
        }
    }

    0
}

#[cfg(target_os = "macos")]
fn memory_usage() -> usize {
    use mach::task::{task_info, task_info_t};
    use mach::task_info::TASK_VM_INFO;
    use mach::vm_types::integer_t;
    use mach::{message::mach_msg_type_number_t, traps::mach_task_self};

    let mut info: task_vm_info = unsafe { std::mem::zeroed() };
    let mut count = TASK_VM_INFO_COUNT;

    unsafe {
        let kr = task_info(
            mach_task_self(),
            TASK_VM_INFO,
            &mut info as *mut task_vm_info as task_info_t,
            &mut count as *mut mach_msg_type_number_t,
        );
        if kr == KERN_SUCCESS {
            return info.resident_size as usize;
        }
    }
    0
}

#[cfg(target_os = "windows")]
fn memory_usage() -> usize {
    use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
    use windows::Win32::System::Threading::{GetCurrentProcess, PROCESS_QUERY_INFORMATION};

    let handle = unsafe { GetCurrentProcess() };
    let mut pmc: PROCESS_MEMORY_COUNTERS = unsafe { std::mem::zeroed() };
    
    if unsafe { GetProcessMemoryInfo(handle, &mut pmc, std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32) }.is_ok() {
        return pmc.WorkingSetSize as usize;
    }
    0
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn memory_usage() -> usize {
    // Fallback for unsupported platforms
    0
}

#[cfg(target_os = "linux")]
fn page_size() -> usize {
    use libc::sysconf;
    use libc::_SC_PAGESIZE;
    
    unsafe {
        sysconf(_SC_PAGESIZE) as usize
    }
}
