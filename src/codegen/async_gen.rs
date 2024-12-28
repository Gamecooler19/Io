use inkwell::{
    basic_block::BasicBlock,
    values::{FunctionValue, BasicValueEnum},
};
use crate::{
    ast::{ASTNode, ASTVisitor},
    error::IoError,
    Result,
};

pub struct AsyncTransformer<'ctx> {
    context: &'ctx inkwell::context::Context,
    current_function: Option<FunctionValue<'ctx>>,
    promise_type: inkwell::types::StructType<'ctx>,
}

impl<'ctx> AsyncTransformer<'ctx> {
    pub fn new(context: &'ctx inkwell::context::Context) -> Self {
        // Create Promise type
        let promise_type = context.opaque_struct_type("Promise");
        promise_type.set_body(&[
            context.i64_type().into(), // State
            context.i8_ptr_type().into(), // Data
        ], false);

        Self {
            context,
            current_function: None,
            promise_type,
        }
    }

    pub fn transform_async_function(
        &mut self,
        function: FunctionValue<'ctx>,
        body: &[ASTNode],
    ) -> Result<FunctionValue<'ctx>> {
        // Create state machine structure
        let state_type = self.create_state_machine_type(function, body)?;
        
        // Transform function to return Promise
        let new_fn_type = self.promise_type.fn_type(&[], false);
        let new_function = self.context.module().add_function(
            &format!("{}_async", function.get_name().to_str()?),
            new_fn_type,
            None,
        );

        // Generate state machine implementation
        self.generate_state_machine(new_function, state_type, body)?;

        Ok(new_function)
    }

    fn create_state_machine_type(
        &self,
        function: FunctionValue<'ctx>,
        body: &[ASTNode],
    ) -> Result<inkwell::types::StructType<'ctx>> {
        let mut field_types = vec![
            self.context.i32_type().into(), // Current state
        ];

        // Add captured variables and parameters
        for param in function.get_param_iter() {
            field_types.push(param.get_type());
        }

        let state_type = self.context.opaque_struct_type("AsyncState");
        state_type.set_body(&field_types, false);

        Ok(state_type)
    }

    fn generate_state_machine(
        &mut self,
        function: FunctionValue<'ctx>,
        state_type: inkwell::types::StructType<'ctx>,
        body: &[ASTNode],
    ) -> Result<()> {
        let entry = self.context.append_basic_block(function, "entry");
        let builder = self.context.create_builder();
        builder.position_at_end(entry);

        // Allocate state machine
        let state_ptr = builder.build_alloca(state_type, "state");
        
        // Initialize state to 0
        let state_field_ptr = builder.build_struct_gep(
            state_ptr,
            0,
            "state.field"
        ).unwrap();
        builder.build_store(state_field_ptr, self.context.i32_type().const_int(0, false));

        // Generate state machine jump table
        self.generate_state_transitions(function, state_ptr, body)?;

        Ok(())
    }

    fn generate_state_transitions(
        &mut self,
        function: FunctionValue<'ctx>,
        state_ptr: inkwell::values::PointerValue<'ctx>,
        body: &[ASTNode],
    ) -> Result<()> {
        // Create basic blocks for each state
        let mut state_blocks = Vec::new();
        for (i, _) in body.iter().enumerate() {
            state_blocks.push(self.context.append_basic_block(
                function,
                &format!("state_{}", i)
            ));
        }

        // Generate state machine dispatcher
        let builder = self.context.create_builder();
        let entry = function.get_first_basic_block().unwrap();
        builder.position_at_end(entry);

        // Load current state
        let state_field_ptr = builder.build_struct_gep(
            state_ptr,
            0,
            "state.current"
        ).unwrap();
        let current_state = builder.build_load(state_field_ptr, "current_state");

        // Create switch instruction
        builder.build_switch(
            current_state.into_int_value(),
            state_blocks[0],
            &state_blocks.iter().enumerate().map(|(i, block)| {
                (self.context.i32_type().const_int(i as u64, false), *block)
            }).collect::<Vec<_>>(),
        );

        Ok(())
    }
}
