use std::path::PathBuf;
use crate::error::Result;

#[derive(Debug, Clone, Copy)]
pub enum Target {
    Native,
    Wasm32,
    X86_64Linux,
    X86_64Windows,
    Aarch64,
}

impl Target {
    pub fn from_triple(triple: &str) -> Option<Self> {
        match triple {
            "wasm32-unknown-unknown" => Some(Self::Wasm32),
            "x86_64-unknown-linux-gnu" => Some(Self::X86_64Linux),
            "x86_64-pc-windows-msvc" => Some(Self::X86_64Windows),
            "aarch64-unknown-linux-gnu" => Some(Self::Aarch64),
            _ => None,
        }
    }

    pub fn get_target_triple(&self) -> &'static str {
        match self {
            Self::Wasm32 => "wasm32-unknown-unknown",
            Self::X86_64Linux => "x86_64-unknown-linux-gnu",
            Self::X86_64Windows => "x86_64-pc-windows-msvc",
            Self::Aarch64 => "aarch64-unknown-linux-gnu",
            Self::Native => std::env::consts::ARCH,
        }
    }
}

#[derive(Debug)]
pub struct BuildConfig {
    target: Target,
    optimization_level: OptimizationLevel,
    debug_info: bool,
    pub output_dir: PathBuf,
    pub source_files: Vec<PathBuf>,
    pub emit_ir: bool,
    pub strip_symbols: bool,
}

impl BuildConfig {
    pub fn new(target: Target) -> Self {
        Self {
            target,
            optimization_level: OptimizationLevel::Default,
            debug_info: true,
            output_dir: PathBuf::new(),
            source_files: Vec::new(),
            emit_ir: false,
            strip_symbols: false,
        }
    }

    pub fn build(&self, package: &Package) -> Result<()> {
        // Verify target compatibility first
        self.verify_target_compatibility()?;

        // Compile source files
        let compiled_files = package.compile_sources(self.optimization_level)?;

        // Link dependencies
        let linked_output = if self.debug_info {
            package.link_with_debug(&compiled_files)?
        } else {
            package.link(&compiled_files)?
        };

        // Generate output based on target
        match self.target {
            Target::Wasm32 => package.emit_wasm(linked_output)?,
            Target::Native | Target::X86_64Linux | Target::X86_64Windows | Target::Aarch64 => {
            package.emit_binary(linked_output)?
            }
        };
        Ok(())
    }

    pub fn verify_target_compatibility(&self) -> Result<()> {
        match self.target {
            Target::Wasm32 => {
                if cfg!(not(target_arch = "wasm32")) {
                    bail!("WASM target requires wasm32 target architecture")
                }
            }
            Target::X86_64Linux => {
                if cfg!(not(all(target_arch = "x86_64", target_os = "linux"))) {
                    bail!("Linux x86_64 target requires matching host architecture")
                }
            }
            // ...similar checks for other targets...
        }
        Ok(())
    }

    pub fn with_optimization(&mut self, level: OptimizationLevel) -> &mut Self {
        self.optimization_level = level;
        self
    }

    pub fn with_debug(&mut self, debug: bool) -> &mut Self {
        self.debug_info = debug;
        self
    }
}