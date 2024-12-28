use clap::{Parser, Subcommand};
use inkwell::context::Context;
use io_lang::{compiler::Compiler, Result};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "io")]
#[command(about = "Io Programming Language Compiler")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Build {
        #[arg(short, long)]
        input: PathBuf,

        #[arg(short, long)]
        output: PathBuf,

        #[arg(short, long, default_value = "false")]
        release: bool,
    },
    Run {
        #[arg(short, long)]
        file: PathBuf,

        #[arg(short, long)]
        args: Vec<String>,
    },
    Test {
        #[arg(short, long)]
        path: PathBuf,

        #[arg(short, long)]
        filter: Option<String>,

        #[arg(short, long)]
        parallel: bool,
    },
    Format {
        #[arg(short, long)]
        path: PathBuf,

        #[arg(short, long)]
        check: bool,
    },
}

fn main() -> Result<()> {
    // Initialize logging
    io_lang::init_logging();

    let cli = Cli::parse();
    let context = Context::create();

    match cli.command {
        Commands::Build {
            input,
            output,
            release,
        } => {
            let mut compiler = Compiler::new(&context);
            if release {
                compiler.set_optimization_level(OptimizationLevel::Aggressive);
                compiler.enable_lto(true);
            }
            compiler
                .with_target_triple("x86_64-unknown-linux-gnu")?
                .with_debuginfo(true)
                .with_metrics(true)
                .compile(input, output)?;

            println!("Build completed successfully!");
        }
        Commands::Run { file, args } => {
            let executor = Executor::new();
            executor.run_file(file, args)?;
        }
        Commands::Test {
            path,
            filter,
            parallel,
        } => {
            let mut test_runner = TestRunner::new();
            if parallel {
                test_runner.enable_parallel();
            }
            if let Some(f) = filter {
                test_runner.with_filter(f);
            }
            test_runner.run_tests(path)?;
        }
        Commands::Format { path, check } => {
            let formatter = Formatter::new();
            if check {
                formatter.check(path)?;
            } else {
                formatter.format(path)?;
            }
        }
    }

    Ok(())
}

struct Executor {
    runtime: Runtime,
    context: ExecutionContext,
}

impl Executor {
    fn new() -> Self {
        Self {
            runtime: Runtime::new(),
            context: ExecutionContext::new(),
        }
    }

    fn run_file(&self, path: PathBuf, args: Vec<String>) -> Result<()> {
        let code = std::fs::read_to_string(path)?;

        self.context.set_args(args);
        self.runtime.execute(&code, &self.context)
    }
}

struct TestRunner {
    parallel: bool,
    filter: Option<String>,
    metrics: TestMetrics,
}

impl TestRunner {
    fn new() -> Self {
        Self {
            parallel: false,
            filter: None,
            metrics: TestMetrics::default(),
        }
    }

    fn enable_parallel(&mut self) {
        self.parallel = true;
    }

    fn with_filter<S: Into<String>>(&mut self, filter: S) {
        self.filter = Some(filter.into());
    }

    fn run_tests(&mut self, path: PathBuf) -> Result<()> {
        let test_files = self.collect_test_files(&path)?;

        if self.parallel {
            self.run_parallel_tests(test_files)
        } else {
            self.run_sequential_tests(test_files)
        }
    }

    fn collect_test_files(&self, path: &PathBuf) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        if path.is_dir() {
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "io") {
                    if let Some(filter) = &self.filter {
                        if path.to_string_lossy().contains(filter) {
                            files.push(path);
                        }
                    } else {
                        files.push(path);
                    }
                }
            }
        } else if path.is_file() {
            files.push(path.clone());
        }
        Ok(files)
    }

    fn run_parallel_tests(&mut self, files: Vec<PathBuf>) -> Result<()> {
        use rayon::prelude::*;

        let results: Vec<Result<()>> = files
            .par_iter()
            .map(|file| self.run_single_test(file))
            .collect();

        results.into_iter().collect()
    }

    fn run_sequential_tests(&mut self, files: Vec<PathBuf>) -> Result<()> {
        for file in files {
            self.run_single_test(&file)?;
        }
        Ok(())
    }
}

struct Formatter {
    config: FormattingConfig,
}

impl Formatter {
    fn new() -> Self {
        Self {
            config: FormattingConfig::default(),
        }
    }

    fn format(&self, path: PathBuf) -> Result<()> {
        if path.is_dir() {
            self.format_directory(path)
        } else {
            self.format_file(path)
        }
    }

    fn check(&self, path: PathBuf) -> Result<()> {
        if path.is_dir() {
            self.check_directory(path)
        } else {
            self.check_file(path)
        }
    }

    fn format_directory(&self, dir: PathBuf) -> Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "io") {
                self.format_file(path)?;
            }
        }
        Ok(())
    }

    fn check_directory(&self, dir: PathBuf) -> Result<()> {
        let mut has_errors = false;
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "io") {
                if let Err(e) = self.check_file(path.clone()) {
                    has_errors = true;
                    eprintln!("Formatting error in {}: {}", path.display(), e);
                }
            }
        }
        if has_errors {
            Err(io_lang::Error::FormattingError)
        } else {
            Ok(())
        }
    }

    fn format_file(&self, path: PathBuf) -> Result<()> {
        let code = std::fs::read_to_string(&path)?;
        let formatted = self.format_code(&code)?;
        std::fs::write(&path, formatted)?;
        Ok(())
    }

    fn check_file(&self, path: PathBuf) -> Result<()> {
        let code = std::fs::read_to_string(&path)?;
        let formatted = self.format_code(&code)?;
        if code != formatted {
            Err(io_lang::Error::FormattingError)
        } else {
            Ok(())
        }
    }

    fn format_code(&self, code: &str) -> Result<String> {
        let parser = Parser::new(code);
        let ast = parser.parse()?;
        let formatter = CodeFormatter::new(&self.config);
        Ok(formatter.format(&ast))
    }
}
