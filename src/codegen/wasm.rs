use inkwell::{
    module::Module,
    targets::{
        FileType, InitializationConfig, Target,
        TargetMachine, TargetTriple,
    },
    OptimizationLevel,
};
use std::path::Path;
use crate::{error::IoError, Result};

pub struct WasmGenerator<'ctx> {
    context: &'ctx inkwell::context::Context,
    module: Module<'ctx>,
}

impl<'ctx> WasmGenerator<'ctx> {
    pub fn new(context: &'ctx inkwell::context::Context, module_name: &str) -> Result<Self> {
        // Initialize WASM target
        Target::initialize_webassembly(&InitializationConfig::default())?;

        let triple = TargetTriple::create("wasm32-unknown-unknown");
        let target = Target::from_triple(&triple)
            .map_err(|e| IoError::runtime_error(format!("Failed to get target: {}", e)))?;

        let module = context.create_module(module_name);
        module.set_triple(&triple);

        Ok(Self { context, module })
    }

    pub fn emit_wasm<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let target_machine = Target::from_name("wasm32")
            .ok_or_else(|| IoError::runtime_error("Failed to get wasm32 target"))?
            .create_target_machine(
                &TargetTriple::create("wasm32-unknown-unknown"),
                "generic",
                "",
                OptimizationLevel::Default,
                inkwell::targets::RelocMode::Default,
                inkwell::targets::CodeModel::Default,
            )
            .ok_or_else(|| IoError::runtime_error("Failed to create target machine"))?;

        target_machine
            .write_to_file(&self.module, FileType::Object, path.as_ref())
            .map_err(|e| IoError::runtime_error(format!("Failed to write WASM: {}", e)))?;

        Ok(())
    }

    pub fn add_wasm_export(&mut self, name: &str, function: inkwell::values::FunctionValue<'ctx>) {
        let export_name = format!("__wasm_export_{}", name);
        function.set_name(&export_name);
    }
}
