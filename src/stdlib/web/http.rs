use inkwell::values::FunctionValue;
use crate::{Result, error::IoError};

pub struct HttpFunctions<'ctx> {
    get_fn: FunctionValue<'ctx>,
    post_fn: FunctionValue<'ctx>,
    put_fn: FunctionValue<'ctx>,
    delete_fn: FunctionValue<'ctx>,
}

impl<'ctx> HttpFunctions<'ctx> {
    pub fn new(codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>) -> Result<Self> {
        // Create HTTP method functions
        let get_fn = Self::create_http_method(codegen, "http_get")?;
        let post_fn = Self::create_http_method(codegen, "http_post")?;
        let put_fn = Self::create_http_method(codegen, "http_put")?;
        let delete_fn = Self::create_http_method(codegen, "http_delete")?;

        Ok(Self {
            get_fn,
            post_fn,
            put_fn,
            delete_fn,
        })
    }

    fn create_http_method(
        codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>,
        name: &str,
    ) -> Result<FunctionValue<'ctx>> {
        let http_response_type = codegen.get_type("HttpResponse")?;
        let fn_type = http_response_type.fn_type(&[
            codegen.string_type().into(), // URL
            codegen.string_type().into(), // Headers
            codegen.string_type().into(), // Body (optional)
        ], false);

        Ok(codegen.module.add_function(name, fn_type, None))
    }

    pub fn generate_bindings(&self, codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>) -> Result<()> {
        // Generate Io language bindings for HTTP functions
        self.generate_get_binding(codegen)?;
        self.generate_post_binding(codegen)?;
        self.generate_put_binding(codegen)?;
        self.generate_delete_binding(codegen)?;
        Ok(())
    }

    fn generate_get_binding(&self, codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>) -> Result<()> {
        // Create high-level Io function for HTTP GET
        let builder = codegen.context.create_builder();
        let fn_type = self.get_fn.get_type();
        let function = codegen.module.add_function("get", fn_type, None);
        
        let entry = codegen.context.append_basic_block(function, "entry");
        builder.position_at_end(entry);

        // Add error handling and response processing
        let result = builder.build_call(
            self.get_fn,
            &[
                function.get_nth_param(0).unwrap().into(),
                function.get_nth_param(1).unwrap().into(),
                codegen.string_type().const_string("", false).into(),
            ],
            "get_result",
        );

        builder.build_return(Some(&result.try_as_basic_value().left().unwrap()));
        Ok(())
    }

    fn generate_post_binding(&self, codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>) -> Result<()> {
        let builder = codegen.context.create_builder();
        let fn_type = self.post_fn.get_type();
        let function = codegen.module.add_function("post", fn_type, None);

        let entry = codegen.context.append_basic_block(function, "entry");
        let validation = codegen.context.append_basic_block(function, "validation");
        let send_request = codegen.context.append_basic_block(function, "send_request");
        let error_block = codegen.context.append_basic_block(function, "error");

        // Entry block - validate inputs
        builder.position_at_end(entry);
        let body = function.get_nth_param(2).unwrap();
        let has_body = builder.build_is_not_null(
            body.into_pointer_value(),
            "has_body",
        );
        builder.build_conditional_branch(has_body, validation, error_block);

        // Validation block
        builder.position_at_end(validation);
        let content_type = builder.build_call(
            codegen.module.get_function("get_content_type").unwrap(),
            &[function.get_nth_param(1).unwrap().into()],
            "content_type",
        );
        let is_valid = builder.build_is_not_null(
            content_type.try_as_basic_value().left().unwrap().into_pointer_value(),
            "is_valid",
        );
        builder.build_conditional_branch(is_valid, send_request, error_block);

        // Send request block
        builder.position_at_end(send_request);
        let result = builder.build_call(
            self.post_fn,
            &[
                function.get_nth_param(0).unwrap().into(),
                function.get_nth_param(1).unwrap().into(),
                body.into(),
            ],
            "post_result",
        );
        builder.build_return(Some(&result.try_as_basic_value().left().unwrap()));

        // Error block
        builder.position_at_end(error_block);
        let error_response = self.build_error_response(builder, codegen, "Invalid POST request");
        builder.build_return(Some(&error_response));

        Ok(())
    }

    fn generate_put_binding(&self, codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>) -> Result<()> {
        let builder = codegen.context.create_builder();
        let fn_type = self.put_fn.get_type();
        let function = codegen.module.add_function("put", fn_type, None);

        let entry = codegen.context.append_basic_block(function, "entry");
        let validation = codegen.context.append_basic_block(function, "validation");
        let check_exists = codegen.context.append_basic_block(function, "check_exists");
        let send_request = codegen.context.append_basic_block(function, "send_request");
        let error_block = codegen.context.append_basic_block(function, "error");

        // Entry and validation
        builder.position_at_end(entry);
        let url = function.get_nth_param(0).unwrap();
        let resource_exists = builder.build_call(
            codegen.module.get_function("check_resource_exists").unwrap(),
            &[url.into()],
            "exists",
        );
        builder.build_conditional_branch(
            resource_exists.try_as_basic_value().left().unwrap().into_int_value(),
            validation,
            error_block,
        );

        // Validate request body
        builder.position_at_end(validation);
        let body = function.get_nth_param(2).unwrap();
        let is_valid = builder.build_call(
            codegen.module.get_function("validate_put_body").unwrap(),
            &[body.into()],
            "is_valid",
        );
        builder.build_conditional_branch(
            is_valid.try_as_basic_value().left().unwrap().into_int_value(),
            send_request,
            error_block,
        );

        // Send PUT request
        builder.position_at_end(send_request);
        let result = builder.build_call(
            self.put_fn,
            &[url.into(), function.get_nth_param(1).unwrap().into(), body.into()],
            "put_result",
        );
        builder.build_return(Some(&result.try_as_basic_value().left().unwrap()));

        // Error handling
        builder.position_at_end(error_block);
        let error_response = self.build_error_response(builder, codegen, "Invalid PUT request");
        builder.build_return(Some(&error_response));

        Ok(())
    }

    fn generate_delete_binding(&self, codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>) -> Result<()> {
        let builder = codegen.context.create_builder();
        let fn_type = self.delete_fn.get_type();
        let function = codegen.module.add_function("delete", fn_type, None);

        let entry = codegen.context.append_basic_block(function, "entry");
        let confirm = codegen.context.append_basic_block(function, "confirm");
        let send_request = codegen.context.append_basic_block(function, "send_request");
        let error_block = codegen.context.append_basic_block(function, "error");

        // Entry - check resource exists
        builder.position_at_end(entry);
        let url = function.get_nth_param(0).unwrap();
        let resource_exists = builder.build_call(
            codegen.module.get_function("check_resource_exists").unwrap(),
            &[url.into()],
            "exists",
        );
        builder.build_conditional_branch(
            resource_exists.try_as_basic_value().left().unwrap().into_int_value(),
            confirm,
            error_block,
        );

        // Confirm deletion
        builder.position_at_end(confirm);
        let confirmed = builder.build_call(
            codegen.module.get_function("confirm_deletion").unwrap(),
            &[url.into()],
            "confirmed",
        );
        builder.build_conditional_branch(
            confirmed.try_as_basic_value().left().unwrap().into_int_value(),
            send_request,
            error_block,
        );

        // Send DELETE request
        builder.position_at_end(send_request);
        let result = builder.build_call(
            self.delete_fn,
            &[
                url.into(),
                function.get_nth_param(1).unwrap().into(),
                codegen.string_type().const_string("", false).into(),
            ],
            "delete_result",
        );
        builder.build_return(Some(&result.try_as_basic_value().left().unwrap()));

        // Error handling
        builder.position_at_end(error_block);
        let error_response = self.build_error_response(builder, codegen, "Invalid DELETE request");
        builder.build_return(Some(&error_response));

        Ok(())
    }

    fn build_error_response(
        &self,
        builder: &inkwell::builder::Builder<'ctx>,
        codegen: &crate::codegen::llvm::LLVMCodeGen<'ctx>,
        message: &str,
    ) -> inkwell::values::BasicValueEnum<'ctx> {
        let response_type = codegen.get_type("HttpResponse").unwrap();
        let response = builder.build_alloca(response_type.into_struct_type(), "error_response");

        // Set error status code (400)
        let status_ptr = builder.build_struct_gep(response, 0, "status_ptr").unwrap();
        builder.build_store(status_ptr, codegen.context.i32_type().const_int(400, false));

        // Set error message
        let msg_ptr = builder.build_struct_gep(response, 1, "msg_ptr").unwrap();
        let error_msg = builder.build_global_string_ptr(message, "error_msg");
        builder.build_store(msg_ptr, error_msg);

        // Set empty body
        let body_ptr = builder.build_struct_gep(response, 2, "body_ptr").unwrap();
        builder.build_store(body_ptr, codegen.string_type().const_string("", false));

        builder.build_load(response, "response")
    }
}
