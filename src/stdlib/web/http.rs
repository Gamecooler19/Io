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

    // TODO: Similar implementations for post, put, delete bindings...
}
