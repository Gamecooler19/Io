use clap::{Parser, Subcommand};
use io_lang::{build::BuildConfig, Result};
use std::{fs, path::PathBuf, process::Command};

#[derive(Parser)]
#[command(name = "ioc")]
#[command(about = "Io Language Compiler")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    New {
        #[arg(short, long)]
        name: String,
    },
    Build {
        #[arg(short, long)]
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(short, long)]
        release: bool,
    },
    Run {
        #[arg(short, long)]
        file: PathBuf,
    },
    Test {
        #[arg(short, long)]
        path: PathBuf,
    },
    Doc {
        #[arg(short, long)]
        output: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::New { name } => {
            println!("Creating new project: {}", name);
            let project_dir = PathBuf::from(&name);

            // Create project directory structure
            fs::create_dir_all(&project_dir)?;
            fs::create_dir_all(project_dir.join("src"))?;
            fs::create_dir_all(project_dir.join("tests"))?;

            // Create basic project files
            fs::write(
                project_dir.join("src/main.io"),
                b"fn main() {\n    println!(\"Hello from Io!\")\n}\n",
            )?;

            // Create project configuration
            fs::write(
                project_dir.join("io.toml"),
                format!("name = \"{}\"\nversion = \"0.1.0\"\n", name).as_bytes(),
            )?;

            println!("✅ Project created successfully!");
            Ok(())
        }
        Commands::Build {
            input,
            output,
            release,
        } => {
            println!("Building project...");
            let config = BuildConfig {
                input_path: input.clone(),
                output_path: output.clone(),
                release_mode: release,
                optimization_level: if release { 3 } else { 0 },
            };

            // Perform the build
            io_lang::build::build(&config)?;

            println!("✅ Build completed successfully!");
            Ok(())
        }
        Commands::Run { file } => {
            println!("Running project...");

            // First build the project to a temporary location
            let temp_output = std::env::temp_dir().join("io_temp_executable");
            let config = BuildConfig {
                input_path: file,
                output_path: temp_output.clone(),
                release_mode: false,
                optimization_level: 0,
            };

            io_lang::build::build(&config)?;

            // Execute the compiled program
            let status = Command::new(temp_output)
                .status()
                .expect("Failed to execute program");

            if !status.success() {
                return Err("Program execution failed".into());
            }

            Ok(())
        }
        Commands::Test { path } => {
            println!("Running tests in: {}", path.display());

            // Find all test files
            let test_files = fs::read_dir(path)?
                .filter_map(Result::ok)
                .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "io"));

            let mut failed_tests = 0;
            let mut passed_tests = 0;

            // Run each test file
            for test_file in test_files {
                println!("Testing: {}", test_file.path().display());
                match run_test(&test_file.path()) {
                    Ok(_) => {
                        println!("✅ Test passed");
                        passed_tests += 1;
                    }
                    Err(e) => {
                        println!("❌ Test failed: {}", e);
                        failed_tests += 1;
                    }
                }
            }

            println!(
                "\nTest Results: {} passed, {} failed",
                passed_tests, failed_tests
            );
            if failed_tests > 0 {
                return Err("Some tests failed".into());
            }

            Ok(())
        }
        Commands::Doc { output } => {
            println!("Generating documentation...");

            // Create documentation directory
            fs::create_dir_all(&output)?;

            // Generate documentation using the io_lang documentation generator
            io_lang::doc::generate_docs(std::env::current_dir()?, output)?;

            println!("✅ Documentation generated successfully!");
            Ok(())
        }
    }
}

fn run_test(test_path: &PathBuf) -> Result<()> {
    // Build and run test file
    let temp_output = std::env::temp_dir().join("io_test_executable");
    let config = BuildConfig {
        input_path: test_path.clone(),
        output_path: temp_output.clone(),
        release_mode: false,
        optimization_level: 0,
    };

    io_lang::build::build(&config)?;

    let status = Command::new(temp_output)
        .status()
        .expect("Failed to execute test");

    if !status.success() {
        return Err("Test execution failed".into());
    }

    Ok(())
}
