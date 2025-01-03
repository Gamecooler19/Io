use inkwell::values::{FunctionValue, BasicValueEnum};
use crate::{Result, error::IoError};

pub struct WebSocketModule<'ctx> {
    ws_type: inkwell::types::StructType<'ctx>,
    connect_fn: FunctionValue<'ctx>,
    send_fn: FunctionValue<'ctx>,
    receive_fn: FunctionValue<'ctx>,
    on_message_fn: FunctionValue<'ctx>,
    close_fn: FunctionValue<'ctx>,
}

impl<'ctx> WebSocketModule<'ctx> {
    pub fn new(codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>) -> Result<Self> {
        let context = codegen.context;
        let ws_type = context.opaque_struct_type("WebSocket");
        
        // Define WebSocket structure
        ws_type.set_body(&[
            context.i8_ptr_type().into(),  // URL
            context.i8_ptr_type().into(),  // Protocol
            context.i32_type().into(),     // State
            context.i8_ptr_type().into(),  // Message callback
        ], false);

        // Create WebSocket functions
        let connect_fn = Self::create_connect_function(codegen, ws_type)?;
        let send_fn = Self::create_send_function(codegen, ws_type)?;
        let receive_fn = Self::create_receive_function(codegen, ws_type)?;
        let on_message_fn = Self::create_on_message_function(codegen, ws_type)?;
        let close_fn = Self::create_close_function(codegen, ws_type)?;

        Ok(Self {
            ws_type,
            connect_fn,
            send_fn,
            receive_fn,
            on_message_fn,
            close_fn,
        })
    }

    fn create_connect_function(
        codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>,
        ws_type: inkwell::types::StructType<'ctx>,
    ) -> Result<FunctionValue<'ctx>> {
        let fn_type = ws_type.fn_type(&[
            codegen.string_type().into(),  // URL
            codegen.string_type().into(),  // Protocol
        ], false);

        Ok(codegen.module.add_function("ws_connect", fn_type, None))
    }

    pub fn generate_bindings(&self, codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>) -> Result<()> {
        // Generate Io language bindings for WebSocket
        self.generate_connect_binding(codegen)?;
        self.generate_send_binding(codegen)?;
        self.generate_receive_binding(codegen)?;
        Ok(())
    }

    fn generate_connect_binding(&self, codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>) -> Result<()> {
        let builder = codegen.context.create_builder();
        let fn_type = self.connect_fn.get_type();
        let function = codegen.module.add_function("websocket_connect", fn_type, None);

        let entry = codegen.context.append_basic_block(function, "entry");
        let validate_url = codegen.context.append_basic_block(function, "validate_url");
        let setup_connection = codegen.context.append_basic_block(function, "setup_connection");
        let handle_error = codegen.context.append_basic_block(function, "handle_error");
        let return_block = codegen.context.append_basic_block(function, "return");

        // Entry block - validate inputs
        builder.position_at_end(entry);
        let url = function.get_nth_param(0).unwrap();
        let protocol = function.get_nth_param(1).unwrap();

        // Check for null parameters
        let url_null = builder.build_is_null(url.into_pointer_value(), "url_null");
        builder.build_conditional_branch(url_null, handle_error, validate_url);

        // URL validation block
        builder.position_at_end(validate_url);
        let url_valid = builder.build_call(
            codegen.module.get_function("validate_websocket_url").unwrap(),
            &[url.into()],
            "url_valid",
        );

        builder.build_conditional_branch(
            url_valid.try_as_basic_value().left().unwrap().into_int_value(),
            setup_connection,
            handle_error,
        );

        // Connection setup block
        builder.position_at_end(setup_connection);
        let ws_config = self.build_websocket_config(&builder, codegen, url, protocol)?;
        let connection = builder.build_call(
            self.connect_fn,
            &[url.into(), protocol.into()],
            "connection",
        );

        let success = self.build_connection_validation(&builder, connection.try_as_basic_value().left().unwrap())?;
        builder.build_conditional_branch(
            success.into_int_value(),
            return_block,
            handle_error,
        );

        // Error handling block
        builder.position_at_end(handle_error);
        self.build_error_handler(&builder, codegen);
        builder.build_unconditional_branch(return_block);

        // Return block
        builder.position_at_end(return_block);
        let result = builder.build_phi(
            self.ws_type,
            "result",
        );

        result.add_incoming(&[
            (&connection.try_as_basic_value().left().unwrap(), setup_connection),
            (&self.ws_type.const_null(), handle_error),
        ]);

        builder.build_return(Some(&result.as_basic_value()));
        Ok(())
    }

    fn build_websocket_config(
        &self,
        builder: &inkwell::builder::Builder<'ctx>,
        codegen: &crate::codegen::llvm::LLVMCodeGen<'ctx>,
        url: inkwell::values::BasicValueEnum<'ctx>,
        protocol: inkwell::values::BasicValueEnum<'ctx>,
    ) -> Result<inkwell::values::BasicValueEnum<'ctx>> {
        // Create WebSocket configuration
        let config_type = codegen.context.struct_type(
            &[
                codegen.context.i8_type().ptr_type(inkwell::AddressSpace::Generic).into(), // URL
                codegen.context.i8_type().ptr_type(inkwell::AddressSpace::Generic).into(), // Protocol
                codegen.context.i32_type().into(),                                         // Timeout
                codegen.context.bool_type().into(),                                        // Auto Reconnect
                codegen.context.i32_type().into(),                                        // Max Retries
            ],
            false,
        );

        let config = builder.build_alloca(config_type, "ws_config");

        // Set configuration fields
        let url_ptr = builder.build_struct_gep(config, 0, "url_ptr")
            .map_err(|_| IoError::runtime_error("Failed to get URL pointer"))?;
        builder.build_store(url_ptr, url);

        let protocol_ptr = builder.build_struct_gep(config, 1, "protocol_ptr")
            .map_err(|_| IoError::runtime_error("Failed to get protocol pointer"))?;
        builder.build_store(protocol_ptr, protocol);

        // Set default timeout (30 seconds)
        let timeout_ptr = builder.build_struct_gep(config, 2, "timeout_ptr")
            .map_err(|_| IoError::runtime_error("Failed to get timeout pointer"))?;
        builder.build_store(timeout_ptr, codegen.context.i32_type().const_int(30000, false));

        // Enable auto reconnect
        let reconnect_ptr = builder.build_struct_gep(config, 3, "reconnect_ptr")
            .map_err(|_| IoError::runtime_error("Failed to get reconnect pointer"))?;
        builder.build_store(reconnect_ptr, codegen.context.bool_type().const_int(1, false));

        // Set max retries
        let retries_ptr = builder.build_struct_gep(config, 4, "retries_ptr")
            .map_err(|_| IoError::runtime_error("Failed to get retries pointer"))?;
        builder.build_store(retries_ptr, codegen.context.i32_type().const_int(3, false));

        Ok(builder.build_load(config, "config"))
    }

    fn build_connection_validation(
        &self,
        builder: &inkwell::builder::Builder<'ctx>,
        ws_instance: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>> {
        let validation_block = builder.get_insert_block().unwrap();
        let function = validation_block.get_parent().unwrap();

        // Create validation blocks
        let check_state = builder.context.append_basic_block(function, "check_state");
        let check_handshake = builder.context.append_basic_block(function, "check_handshake");
        let validation_success = builder.context.append_basic_block(function, "validation_success");
        let validation_failed = builder.context.append_basic_block(function, "validation_failed");

        // Check connection state
        builder.build_unconditional_branch(check_state);
        builder.position_at_end(check_state);

        let state_ptr = builder.build_struct_gep(
            ws_instance.into_pointer_value(),
            2,
            "state_ptr",
        ).unwrap();
        let state = builder.build_load(state_ptr, "state");

        let is_connected = builder.build_int_compare(
            inkwell::IntPredicate::EQ,
            state.into_int_value(),
            builder.context.i32_type().const_int(1, false), // 1 = Connected state
            "is_connected",
        );

        builder.build_conditional_branch(is_connected, check_handshake, validation_failed);

        // Check WebSocket handshake
        builder.position_at_end(check_handshake);
        let handshake_valid = builder.build_call(
            function.get_context().get_type_named("verify_ws_handshake").unwrap(),
            &[ws_instance.into()],
            "handshake_valid",
        );

        builder.build_conditional_branch(
            handshake_valid.try_as_basic_value().left().unwrap().into_int_value(),
            validation_success,
            validation_failed,
        );

        // Success case
        builder.position_at_end(validation_success);
        let success = builder.context.bool_type().const_int(1, false);
        builder.build_return(Some(&success));

        // Failure case
        builder.position_at_end(validation_failed);
        let failure = builder.context.bool_type().const_int(0, false);
        builder.build_return(Some(&failure));

        Ok(success.into())
    }

    fn build_error_handler(
        &self,
        builder: &inkwell::builder::Builder<'ctx>,
        codegen: &crate::codegen::llvm::LLVMCodeGen<'ctx>,
    ) {
        let error_msg = builder.build_global_string_ptr(
            "WebSocket connection failed",
            "error_msg",
        );

        builder.build_call(
            codegen.module.get_function("handle_ws_error").unwrap(),
            &[error_msg.as_pointer_value().into()],
            "error_handled",
        );

        // Log error details
        let log_error = codegen.module.get_function("log_ws_error").unwrap();
        builder.build_call(
            log_error,
            &[error_msg.as_pointer_value().into()],
            "logged",
        );
    }
}
