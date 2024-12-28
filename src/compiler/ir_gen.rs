use inkwell::{
    basic_block::BasicBlock,
    builder::Builder,
    context::Context,
    module::Module,
    passes::PassManager,
    types::BasicTypeEnum,
    values::{BasicValue, FunctionValue, PhiValue},
    OptimizationLevel,
};
use std::collections::HashMap;
use crate::{
    ast::{ASTNode, BinaryOperator},
    compiler::control_flow::ControlFlowGraph,
    error::IoError,
    types::Type,
    Result,
};

pub struct IRGenerator<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    function_pass_manager: PassManager<FunctionValue<'ctx>>,
    variables: HashMap<String, BasicValueEnum<'ctx>>,
    current_function: Option<FunctionValue<'ctx>>,
    optimization_level: OptimizationLevel,
}

impl<'ctx> IRGenerator<'ctx> {
    pub fn new(context: &'ctx Context, module_name: &str) -> Self {
        let module = context.create_module(module_name);
        let builder = context.create_builder();
        let fpm = PassManager::create(&module);

        // Add optimization passes
        fpm.add_instruction_combining_pass();
        fpm.add_reassociate_pass();
        fpm.add_gvn_pass();
        fpm.add_cfg_simplification_pass();
        fpm.add_basic_alias_analysis_pass();
        fpm.add_promote_memory_to_register_pass();
        fpm.initialize();

        Self {
            context,
            module,
            builder,
            function_pass_manager: fpm,
            variables: HashMap::new(),
            current_function: None,
            optimization_level: OptimizationLevel::Default,
        }
    }

    pub fn generate_ir(&mut self, ast: &ASTNode, cfg: &ControlFlowGraph) -> Result<()> {
        match ast {
            ASTNode::Function { name, params, return_type, body, is_async } => {
                self.generate_function(name, params, return_type, body, cfg, *is_async)
            }
            // Handle other nodes...
            _ => Err(IoError::runtime_error("Unsupported node type for IR generation")),
        }
    }

    fn generate_function(
        &mut self,
        name: &str,
        params: &[Parameter],
        return_type: &Option<String>,
        body: &[ASTNode],
        cfg: &ControlFlowGraph,
        is_async: bool,
    ) -> Result<()> {
        let ret_type = self.get_llvm_type(return_type.as_deref().unwrap_or("unit"))?;
        let param_types: Vec<_> = params
            .iter()
            .map(|p| self.get_llvm_type(&p.type_annotation))
            .collect::<Result<_>>()?;

        let fn_type = ret_type.fn_type(&param_types, false);
        let function = self.module.add_function(name, fn_type, None);

        // Create basic blocks for CFG
        let mut blocks = HashMap::new();
        for (&id, block) in &cfg.blocks {
            blocks.insert(id, self.context.append_basic_block(function, &format!("block_{}", id)));
        }

        // Generate code for each basic block
        for (&id, block) in &cfg.blocks {
            let llvm_block = blocks[&id];
            self.builder.position_at_end(llvm_block);

            // Generate code for statements
            for stmt in &block.statements {
                self.generate_statement(stmt)?;
            }

            // Add branch instructions
            if block.successors.is_empty() {
                // Return void if no explicit return
                self.builder.build_return(None);
            } else if block.successors.len() == 1 {
                self.builder.build_unconditional_branch(blocks[&block.successors[0]]);
            }
            // Conditional branches handled in statement generation
        }

        // Optimize function
        if self.optimization_level != OptimizationLevel::None {
            self.function_pass_manager.run_on(&function);
        }

        Ok(())
    }

    fn get_llvm_type(&self, type_name: &str) -> Result<BasicTypeEnum<'ctx>> {
        match type_name {
            "int" => Ok(self.context.i64_type().into()),
            "float" => Ok(self.context.f64_type().into()),
            "bool" => Ok(self.context.bool_type().into()),
            // Add more types...
            _ => Err(IoError::type_error(format!("Unsupported type: {}", type_name))),
        }
    }

    fn generate_statement(&mut self, stmt: &ASTNode) -> Result<()> {
        match stmt {
            ASTNode::VariableDeclaration { name, type_annotation, value, is_mutable } => {
                self.generate_variable_declaration(name, type_annotation, value, *is_mutable)
            }
            ASTNode::Assignment { target, value } => {
                self.generate_assignment(target, value)
            }
            ASTNode::If { condition, then_branch, else_branch } => {
                self.generate_if_statement(condition, then_branch, else_branch)
            }
            ASTNode::While { condition, body } => {
                self.generate_while_loop(condition, body)
            }
            ASTNode::Try { body, catch_clauses, finally } => {
                self.generate_try_catch(body, catch_clauses, finally)
            }
            _ => Err(IoError::runtime_error("Unsupported statement type")),
        }
    }

    fn generate_if_statement(
        &mut self,
        condition: &ASTNode,
        then_branch: &[ASTNode],
        else_branch: &Option<Vec<ASTNode>>,
    ) -> Result<()> {
        let condition_value = self.generate_expression(condition)?;
        let function = self.current_function.ok_or_else(|| {
            IoError::runtime_error("No current function")
        })?;

        let then_block = self.context.append_basic_block(function, "then");
        let else_block = self.context.append_basic_block(function, "else");
        let merge_block = self.context.append_basic_block(function, "merge");

        self.builder.build_conditional_branch(
            condition_value.into_int_value(),
            then_block,
            else_block,
        );

        // Generate then branch
        self.builder.position_at_end(then_block);
        for stmt in then_branch {
            self.generate_statement(stmt)?;
        }
        self.builder.build_unconditional_branch(merge_block);

        // Generate else branch
        self.builder.position_at_end(else_block);
        if let Some(else_stmts) = else_branch {
            for stmt in else_stmts {
                self.generate_statement(stmt)?;
            }
        }
        self.builder.build_unconditional_branch(merge_block);

        // Continue at merge block
        self.builder.position_at_end(merge_block);
        Ok(())
    }

    fn generate_try_catch(
        &mut self,
        body: &[ASTNode],
        catch_clauses: &[CatchClause],
        finally: &Option<Vec<ASTNode>>,
    ) -> Result<()> {
        let function = self.current_function.ok_or_else(|| {
            IoError::runtime_error("No current function")
        })?;

        // Create landing pad for exceptions
        let landing_pad = self.context.append_basic_block(function, "landing_pad");
        let finally_block = self.context.append_basic_block(function, "finally");
        let resume_block = self.context.append_basic_block(function, "resume");

        // Generate try body with exception handling
        self.generate_protected_region(body, landing_pad)?;

        // Generate catch clauses
        self.builder.position_at_end(landing_pad);
        for catch in catch_clauses {
            self.generate_catch_clause(catch)?;
        }

        // Generate finally block
        self.builder.position_at_end(finally_block);
        if let Some(finally_stmts) = finally {
            for stmt in finally_stmts {
                self.generate_statement(stmt)?;
            }
        }
        self.builder.build_unconditional_branch(resume_block);

        // Continue at resume block
        self.builder.position_at_end(resume_block);
        Ok(())
    }

    fn generate_protected_region(
        &mut self,
        body: &[ASTNode],
        landing_pad: BasicBlock<'ctx>,
    ) -> Result<()> {
        // Set up exception handling metadata
        let personality_fn = self.module.get_function("__gxx_personality_v0")
            .ok_or_else(|| IoError::runtime_error("Personality function not found"))?;

        // Generate exception handling tables
        self.builder.build_invoke(
            personality_fn,
            &[],
            landing_pad,
            self.builder.get_insert_block().unwrap(),
            "invoke",
        );

        // Generate protected body
        for stmt in body {
            self.generate_statement(stmt)?;
        }

        Ok(())
    }

    fn generate_catch_clause(&mut self, catch: &CatchClause) -> Result<()> {
        // Generate type info for caught exception
        let type_info = self.get_type_info(&catch.error_type)?;

        // Set up catch block
        let catch_block = self.context.append_basic_block(
            self.current_function.unwrap(),
            "catch",
        );
        self.builder.position_at_end(catch_block);

        // Store exception in binding
        let exception_ptr = self.builder.build_alloca(
            self.context.i8_type().ptr_type(Default::default()),
            &catch.binding,
        );
        self.variables.insert(catch.binding.clone(), exception_ptr.into());

        // Generate catch body
        for stmt in &catch.body {
            self.generate_statement(stmt)?;
        }

        Ok(())
    }
}
