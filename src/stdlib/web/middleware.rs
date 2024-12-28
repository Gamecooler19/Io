use std::collections::VecDeque;
use inkwell::values::FunctionValue;
use crate::{Result, error::IoError};

#[derive(Clone)]
pub struct Middleware<'ctx> {
    function: FunctionValue<'ctx>,
    name: String,
    priority: i32,
}

pub struct MiddlewareChain<'ctx> {
    middlewares: VecDeque<Middleware<'ctx>>,
    context: &'ctx inkwell::context::Context,
}

impl<'ctx> MiddlewareChain<'ctx> {
    pub fn new(context: &'ctx inkwell::context::Context) -> Self {
        Self {
            middlewares: VecDeque::new(),
            context,
        }
    }

    pub fn add(&mut self, middleware: Middleware<'ctx>) {
        let position = self.middlewares
            .iter()
            .position(|m| m.priority > middleware.priority)
            .unwrap_or(self.middlewares.len());
        
        self.middlewares.insert(position, middleware);
    }

    pub fn generate_chain(&self, codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>) -> Result<FunctionValue<'ctx>> {
        let builder = codegen.context.create_builder();
        
        // Create main chain function
        let fn_type = codegen.get_type("HttpResponse")?.fn_type(
            &[codegen.get_type("HttpRequest")?.into()],
            false
        );
        
        let chain_fn = codegen.module.add_function("middleware_chain", fn_type, None);
        let entry = codegen.context.append_basic_block(chain_fn, "entry");
        builder.position_at_end(entry);

        // Generate middleware chain
        let mut current_response = None;
        for middleware in &self.middlewares {
            let response = builder.build_call(
                middleware.function,
                &[
                    chain_fn.get_nth_param(0).unwrap().into(),
                    current_response.unwrap_or_else(|| {
                        codegen.get_type("HttpResponse")?
                            .const_null()
                            .into()
                    }),
                ],
                &format!("{}_response", middleware.name),
            );
            current_response = Some(response.try_as_basic_value().left().unwrap());
        }

        // Return final response
        if let Some(response) = current_response {
            builder.build_return(Some(&response));
        } else {
            builder.build_return(Some(&codegen.get_type("HttpResponse")?.const_null()));
        }

        Ok(chain_fn)
    }
}

pub fn create_middleware<'ctx>(
    codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>,
    name: &str,
    priority: i32,
    handler: FunctionValue<'ctx>,
) -> Result<Middleware<'ctx>> {
    Ok(Middleware {
        function: handler,
        name: name.to_string(),
        priority,
    })
}
