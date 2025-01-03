use crate::error::IoError;
use inkwell::module::Module;
use inkwell::passes::PassManager;
use inkwell::values::FunctionValue;

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
        fpm.add_tail_call_elimination_pass();
        fpm.add_loop_unroll_pass();
        fpm.add_loop_vectorize_pass();
        fpm.add_slp_vectorize_pass();
        fpm.add_aggressive_dce_pass();
        fpm.add_memcpy_optimize_pass();
        fpm.initialize();

        // Module-level optimizations
        mpm.add_dead_arg_elimination_pass();
        mpm.add_function_attrs_pass();
        mpm.add_merge_functions_pass();
        mpm.add_global_dce_pass();
        mpm.add_constant_merge_pass();
        mpm.add_strip_dead_prototypes_pass();
        mpm.add_strip_debug_declare_pass();
        mpm.add_global_optimizer_pass();
        mpm.add_ipsccp_pass();
        mpm.add_prune_eh_pass();
        mpm.add_reassociate_pass();
        mpm.add_instruction_combining_pass();
        mpm.add_cfg_simplification_pass();
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
