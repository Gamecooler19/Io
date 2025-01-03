use crate::error::Result;
use inkwell::{
    module::Module,
    passes::{PassManager, PassManagerBuilder},
    values::FunctionValue,
    OptimizationLevel,
};

pub struct OptimizationPasses<'ctx> {
    module_passes: PassManager<Module<'ctx>>,
    function_passes: PassManager<FunctionValue<'ctx>>,
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

    pub fn run_on_module(&self, module: &Module<'ctx>) -> Result<()> {
        // Run module-level optimizations
        self.module_passes.run_on(module);

        // Run function-level optimizations
        for function in module.get_functions() {
            self.function_passes.run_on(&function);
        }

        Ok(())
    }

    pub fn add_standard_passes(&mut self) {
        // Basic analysis passes
        self.module_passes.add_basic_alias_analysis_pass();
        self.module_passes.add_type_based_alias_analysis_pass();
        self.module_passes.add_sccp_pass();

        // Memory optimizations
        self.module_passes.add_promote_memory_to_register_pass();
        self.module_passes.add_memory_to_register_promotion_pass();
        self.module_passes.add_dead_store_elimination_pass();
        self.module_passes.add_memcpy_optimization_pass();

        // Control flow optimizations
        self.module_passes.add_cfg_simplification_pass();
        self.module_passes.add_jump_threading_pass();
        self.module_passes.add_loop_simplify_pass();
        self.module_passes.add_lcssa_pass();

        // Scalar optimizations
        self.module_passes.add_instruction_combining_pass();
        self.module_passes.add_reassociate_pass();
        self.module_passes.add_gvn_pass();
        self.module_passes.add_early_cse_pass();
        self.module_passes.add_correlated_value_propagation_pass();

        // Dead code elimination
        self.module_passes.add_dead_code_elimination_pass();
        self.module_passes.add_bit_tracking_dce_pass();

        // Function optimizations
        self.function_passes.add_instruction_simplify_pass();
        self.function_passes.add_licm_pass();
        self.function_passes.add_sink_pass();

        // Initialize the function pass manager
        self.function_passes.initialize();
    }

    pub fn add_aggressive_passes(&mut self) {
        self.add_standard_passes();

        // Aggressive inlining
        self.module_passes.add_always_inline_pass();
        self.module_passes.add_partial_inlining_pass();
        self.module_passes.add_hot_cold_splitting_pass();

        // Advanced loop optimizations
        self.module_passes.add_loop_unroll_pass();
        self.module_passes.add_loop_idiom_pass();
        self.module_passes.add_loop_rotate_pass();
        self.module_passes.add_loop_deletion_pass();
        self.module_passes.add_loop_vectorize_pass();
        self.module_passes.add_slp_vectorize_pass();

        // Aggressive scalar optimizations
        self.module_passes.add_aggressive_inst_combine_pass();
        self.module_passes.add_new_gvn_pass();
        self.module_passes.add_delinearization_pass();
        self.module_passes.add_float_to_int_pass();

        // Advanced memory optimizations
        self.module_passes.add_memory_phi_motion_pass();
        self.module_passes.add_mergefunc_pass();
        self.module_passes.add_heap_to_stack_pass();

        // Aggressive dead code elimination
        self.module_passes.add_aggressive_dce_pass();
        self.module_passes.add_dead_arg_elimination_pass();

        // Global optimizations
        self.module_passes.add_global_dce_pass();
        self.module_passes.add_global_optimizer_pass();
        self.module_passes.add_ip_constant_propagation_pass();
        self.module_passes.add_prune_eh_pass();
        self.module_passes.add_rewrite_symbols_pass();

        // Function optimizations
        self.function_passes.add_tail_call_elimination_pass();
        self.function_passes.add_called_value_propagation_pass();
        self.function_passes.add_partially_inline_lib_calls_pass();

        // Cleanup passes
        self.module_passes.add_strip_dead_prototypes_pass();
        self.module_passes.add_strip_debug_declare_pass();
        self.module_passes.add_merge_functions_pass();

        // Initialize both pass managers
        self.function_passes.initialize();
        self.module_passes.initialize();
    }

    // Add helper method for custom optimization pipeline
    pub fn add_custom_passes(&mut self, custom_pipeline: Vec<OptimizationPass<'ctx>>) {
        for pass in custom_pipeline {
            match pass {
                OptimizationPass::Module(pass) => self.module_passes.add_pass(pass),
                OptimizationPass::Function(pass) => self.function_passes.add_pass(pass),
            }
        }

        // Initialize pass managers after adding custom passes
        self.function_passes.initialize();
        self.module_passes.initialize();
    }
}

pub enum Pass<'ctx> {
    Module(PassManager<Module<'ctx>>),
    Function(PassManager<FunctionValue<'ctx>>),
}

pub enum OptimizationPass<'ctx> {
    Module(PassManager<Module<'ctx>>),
    Function(PassManager<FunctionValue<'ctx>>),
}
