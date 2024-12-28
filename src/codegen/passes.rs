use crate::error::Result;
use inkwell::passes::{PassManager, PassManagerBuilder};
use inkwell::OptimizationLevel;

pub struct OptimizationPasses<'ctx> {
    module_passes: PassManager<inkwell::module::Module<'ctx>>,
    function_passes: PassManager<inkwell::values::FunctionValue<'ctx>>,
}

impl<'ctx> OptimizationPasses<'ctx> {
    pub fn new(opt_level: OptimizationLevel) -> Self {
        let module_passes = PassManager::create(());
        let function_passes = PassManager::create(());

        let builder = PassManagerBuilder::create();
        builder.set_optimization_level(opt_level);

        // Set up module-level passes
        builder.populate_module_pass_manager(&module_passes);

        // Set up function-level passes
        builder.populate_function_pass_manager(&function_passes);

        Self {
            module_passes,
            function_passes,
        }
    }

    pub fn run_on_module(&self, module: &inkwell::module::Module<'ctx>) -> Result<()> {
        // Run module-level optimizations
        self.module_passes.run_on(module);

        // Run function-level optimizations
        for function in module.get_functions() {
            self.function_passes.run_on(&function);
        }

        Ok(())
    }

    pub fn add_standard_passes(&mut self) {
        // Add standard optimization passes
        self.module_passes.add_instruction_combining_pass();
        self.module_passes.add_reassociate_pass();
        self.module_passes.add_gvn_pass();
        self.module_passes.add_cfg_simplification_pass();
        self.module_passes.add_basic_alias_analysis_pass();
        self.module_passes.add_promote_memory_to_register_pass();
        self.module_passes.add_dead_store_elimination_pass();
    }

    pub fn add_aggressive_passes(&mut self) {
        self.add_standard_passes();

        // Add more aggressive optimizations
        self.module_passes.add_function_inlining_pass();
        self.module_passes.add_global_dce_pass();
        self.module_passes.add_constant_propagation_pass();
        self.module_passes.add_aggressive_dce_pass();
        self.module_passes.add_tail_call_elimination_pass();
    }
}
