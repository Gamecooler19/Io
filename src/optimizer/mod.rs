use crate::error::IoError;
use inkwell::module::Module;
use inkwell::passes::PassManager;

pub struct Optimizer<'ctx> {
    fpm: PassManager<FunctionValue<'ctx>>,
    mpm: PassManager<Module<'ctx>>,
}

impl<'ctx> Optimizer<'ctx> {
    pub fn new(module: &Module<'ctx>) -> Self {
        let fpm = PassManager::create(module);
        let mpm = PassManager::create(());

        // Add standard optimization passes
        fpm.add_instruction_combining_pass();
        fpm.add_reassociate_pass();
        fpm.add_gvn_pass();
        fpm.add_cfg_simplification_pass();
        fpm.add_basic_alias_analysis_pass();
        fpm.add_promote_memory_to_register_pass();
        fpm.initialize();

        // Module-level optimizations
        mpm.add_dead_arg_elimination_pass();
        mpm.add_function_attrs_pass();
        mpm.add_merge_functions_pass();
        mpm.initialize();

        Self { fpm, mpm }
    }

    pub fn optimize_function(&self, function: &FunctionValue) -> Result<(), IoError> {
        if !self.fpm.run_on(function) {
            return Err(IoError::runtime_error("Function optimization failed"));
        }
        Ok(())
    }

    pub fn optimize_module(&self, module: &Module) -> Result<(), IoError> {
        if !self.mpm.run_on(module) {
            return Err(IoError::runtime_error("Module optimization failed"));
        }
        Ok(())
    }
}
