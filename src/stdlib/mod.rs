use crate::Result;
use inkwell::{context::Context, types::BasicType, values::FunctionValue};

pub mod collections;
pub mod concurrent;
pub mod io;
pub mod network;

use std::collections::HashMap;

pub struct NetworkModule<'ctx> {
    functions: HashMap<String, FunctionValue<'ctx>>,
    context: &'ctx inkwell::context::Context,
}

impl<'ctx> NetworkModule<'ctx> {
    pub fn new(context: &'ctx inkwell::context::Context) -> Self {
        Self {
            functions: std::collections::HashMap::new(),
            context,
        }
    }

    pub fn generate_bindings(
        &mut self,
        codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>,
    ) -> Result<()> {
        self.register_tcp_functions(codegen)?;
        self.register_udp_functions(codegen)?;
        self.register_http_functions(codegen)?;
        Ok(())
    }

    fn register_tcp_functions(
        &mut self,
        codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>,
    ) -> Result<()> {
        let i32_type = self.context.i32_type();
        let i8_ptr_type = self.context.i8_type().ptr_type(Default::default());
        let void_type = self.context.void_type();

        // TCP connect function
        let connect_fn_type = i32_type.fn_type(&[i8_ptr_type.into(), i32_type.into()], false);
        let connect_fn = codegen
            .module
            .add_function("tcp_connect", connect_fn_type, None);
        self.functions.insert("tcp_connect".to_string(), connect_fn);

        // TCP send function
        let send_fn_type = i32_type.fn_type(
            &[i32_type.into(), i8_ptr_type.into(), i32_type.into()],
            false,
        );
        let send_fn = codegen.module.add_function("tcp_send", send_fn_type, None);
        self.functions.insert("tcp_send".to_string(), send_fn);

        Ok(())
    }

    fn register_udp_functions(
        &mut self,
        codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>,
    ) -> Result<()> {
        let i32_type = self.context.i32_type();
        let i8_ptr_type = self.context.i8_type().ptr_type(Default::default());

        // UDP socket creation
        let socket_fn_type = i32_type.fn_type(&[], false);
        let socket_fn = codegen
            .module
            .add_function("udp_socket", socket_fn_type, None);
        self.functions.insert("udp_socket".to_string(), socket_fn);

        // UDP sendto function
        let sendto_fn_type = i32_type.fn_type(
            &[
                i32_type.into(),
                i8_ptr_type.into(),
                i32_type.into(),
                i8_ptr_type.into(),
            ],
            false,
        );
        let sendto_fn = codegen
            .module
            .add_function("udp_sendto", sendto_fn_type, None);
        self.functions.insert("udp_sendto".to_string(), sendto_fn);

        Ok(())
    }

    fn register_http_functions(
        &mut self,
        codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>,
    ) -> Result<()> {
        let i32_type = self.context.i32_type();
        let i8_ptr_type = self.context.i8_type().ptr_type(Default::default());

        // HTTP GET request
        let get_fn_type = i8_ptr_type.fn_type(&[i8_ptr_type.into()], false);
        let get_fn = codegen.module.add_function("http_get", get_fn_type, None);
        self.functions.insert("http_get".to_string(), get_fn);

        // HTTP POST request
        let post_fn_type = i8_ptr_type.fn_type(&[i8_ptr_type.into(), i8_ptr_type.into()], false);
        let post_fn = codegen.module.add_function("http_post", post_fn_type, None);
        self.functions.insert("http_post".to_string(), post_fn);

        Ok(())
    }

    pub fn get_function(&self, name: &str) -> Option<FunctionValue<'ctx>> {
        self.functions.get(name).copied()
    }
}

pub struct StandardLibrary<'ctx> {
    io_module: io::IoModule<'ctx>,
    collections_module: collections::CollectionsModule<'ctx>,
    concurrent_module: concurrent::ConcurrentModule<'ctx>,
    network_module: network::NetworkModule<'ctx>,
}

impl<'ctx> StandardLibrary<'ctx> {
    pub fn new(context: &'ctx Context) -> Result<Self> {
        Ok(Self {
            io_module: io::IoModule::new(),
            collections_module: collections::CollectionsModule::new(context),
            concurrent_module: concurrent::ConcurrentModule::new(context),
            network_module: network::NetworkModule::new(context),
        })
    }

    pub fn initialize(
        &mut self,
        codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>,
    ) -> Result<()> {
        self.io_module.initialize(codegen)?;
        self.collections_module.initialize(codegen)?;
        self.concurrent_module.initialize(codegen)?;
        self.network_module.generate_bindings(codegen)?;
        Ok(())
    }

    pub fn register_all(
        &mut self,
        codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>,
    ) -> Result<()> {
        self.io_module.generate_bindings(codegen)?;
        self.collections_module.generate_bindings(codegen)?;
        self.concurrent_module.generate_bindings(codegen)?;
        self.network_module.generate_bindings(codegen)?;
        Ok(())
    }

    pub fn get_function(&self, name: &str) -> Option<FunctionValue<'ctx>> {
        match name {
            n if n.starts_with("io.") => self.io_module.get_function(&n[3..]),
            n if n.starts_with("collections.") => self.collections_module.get_function(&n[12..]),
            n if n.starts_with("concurrent.") => self.concurrent_module.get_function(&n[11..]),
            n if n.starts_with("network.") => self.network_module.get_function(&n[8..]),
            _ => None,
        }
    }

    pub fn register_builtin_types(
        &self,
        codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>,
    ) -> Result<()> {
        // Register sample built-in types
        let i64_type = codegen.context.i64_type().as_basic_type_enum();
        let f32_type = codegen.context.f32_type().as_basic_type_enum();
        // e.g., store these in codegen if needed
        codegen.register_type("int", i64_type)?;
        codegen.register_type("float", f32_type)?;
        Ok(())
    }

    fn register_basic_types(
        &mut self,
        codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>,
    ) -> Result<()> {
        // Register fundamental types
        let i8_type = codegen.context.i8_type().as_basic_type_enum();
        let i16_type = codegen.context.i16_type().as_basic_type_enum();
        let i32_type = codegen.context.i32_type().as_basic_type_enum();
        let i64_type = codegen.context.i64_type().as_basic_type_enum();
        let f32_type = codegen.context.f32_type().as_basic_type_enum();
        let bool_type = codegen.context.bool_type().as_basic_type_enum();

        // Register all basic types
        codegen.register_type("i8", i8_type)?;
        codegen.register_type("i16", i16_type)?;
        codegen.register_type("i32", i32_type)?;
        codegen.register_type("i64", i64_type)?;
        codegen.register_type("f32", f32_type)?;
        codegen.register_type("bool", bool_type)?;

        Ok(())
    }

    pub fn other_std_functions(
        &mut self,
        codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>,
    ) -> Result<()> {
        let void_type = codegen.context.void_type();
        let i64_type = codegen.context.i64_type();
        let i32_type = codegen.context.i32_type();
        let i8_ptr = codegen.context.i8_type().ptr_type(Default::default());

        // Time functions
        let time_get_fn =
            codegen
                .module
                .add_function("time_get", i64_type.fn_type(&[], false), None);
        self.register_function("time_get", time_get_fn);

        // Random number generation
        let random_int_fn = codegen.module.add_function(
            "random_int",
            i32_type.fn_type(&[i32_type.into(), i32_type.into()], false),
            None,
        );
        self.register_function("random_int", random_int_fn);

        // String manipulation functions
        let strlen_fn =
            codegen
                .module
                .add_function("strlen", i64_type.fn_type(&[i8_ptr.into()], false), None);
        self.register_function("strlen", strlen_fn);

        let strcat_fn = codegen.module.add_function(
            "strcat",
            i8_ptr.fn_type(&[i8_ptr.into(), i8_ptr.into()], false),
            None,
        );
        self.register_function("strcat", strcat_fn);

        Ok(())
    }

    fn register_function(&mut self, name: &str, function: FunctionValue<'ctx>) {
        self.functions.insert(name.to_string(), function);
    }
}
