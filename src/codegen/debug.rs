use inkwell::debug_info::{DICompileUnit, DIScope};
use inkwell::types::*;
use inkwell::values::*;
use inkwell::{context::Context, debug_info::*};

#[derive(Debug)]
pub struct DebugInfo<'ctx> {
    pub compile_unit: DICompileUnit<'ctx>,
    pub current_scope: Option<DIScope<'ctx>>,
    pub builder: inkwell::debug_info::DebugInfoBuilder<'ctx>,
}

#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub file: String,
    pub directory: String,
    pub line: u32,
    pub column: u32,
}

impl SourceLocation {
    pub fn new(file: String, directory: String, line: u32, column: u32) -> Self {
        Self {
            file,
            directory,
            line,
            column,
        }
    }
}

impl<'ctx> DebugInfo<'ctx> {
    pub fn new(
        builder: inkwell::debug_info::DebugInfoBuilder<'ctx>,
        compile_unit: DICompileUnit<'ctx>,
    ) -> Self {
        Self {
            compile_unit,
            current_scope: None,
            builder,
        }
    }

    pub fn set_scope(&mut self, scope: DIScope<'ctx>) {
        self.current_scope = Some(scope);
    }

    pub fn clear_scope(&mut self) {
        self.current_scope = None;
    }

    pub fn initialize(&mut self, module_name: &str, file_name: &str) {
        // Create debug info builder
        let (debug_builder, compile_unit) = self.create_debug_info(module_name, file_name);
        self.builder = Some(debug_builder);
        self.compile_unit = Some(compile_unit);

        // Set up current file
        let file = self.create_file(file_name);
        self.current_file = Some(file);
        self.current_scope = Some(compile_unit.as_debug_info_scope());
    }

    fn create_debug_info(
        &self,
        module_name: &str,
        file_name: &str,
    ) -> (DebugInfoBuilder<'ctx>, DICompileUnit<'ctx>) {
        let (debug_builder, compile_unit) = DebugInfoBuilder::new();

        let compile_unit = debug_builder
            .create_compile_unit(
                DWARFSourceLanguage::C, // Use C as base language
                debug_builder.create_file(file_name, "."),
                "IO Lang Compiler",
                false,
                "",
                0,
                "",
                DWARFEmissionKind::Full,
                0,
                false,
                false,
                "",
                "",
            )
            .unwrap();

        (debug_builder, compile_unit)
    }

    fn create_file(&self, file_name: &str) -> DIFile<'ctx> {
        self.builder.as_ref().unwrap().create_file(file_name, ".")
    }

    pub fn finalize(&self) {
        if let Some(builder) = &self.builder {
            builder.finalize();
        }
    }

    pub fn get_current_scope(&self) -> Option<DIScope<'ctx>> {
        self.current_scope
    }

    pub fn get_current_file(&self) -> Option<DIFile<'ctx>> {
        self.current_file
    }

    pub fn set_debug_location(&self, function: FunctionValue<'ctx>, line: u32, column: u32) {
        if let (Some(builder), Some(scope)) = (&self.builder, &self.current_scope) {
            builder.set_current_debug_location(
                function.get_context(),
                line,
                column,
                scope.clone(),
                None,
            );
        }
    }

    pub fn create_function(
        &self,
        scope: DIScope<'ctx>,
        name: &str,
        linkage_name: Option<&str>,
        file: DIFile<'ctx>,
        line: u32,
    ) -> DISubprogram<'ctx> {
        self.builder
            .as_ref()
            .unwrap()
            .create_function(
                scope,
                name,
                linkage_name,
                file,
                line,
                self.create_subroutine_type(file),
                false,
                true,
                line,
                DIFlags::Public,
                false,
            )
            .unwrap()
    }

    pub fn create_subroutine_type(&self, file: DIFile<'ctx>) -> DISubroutineType<'ctx> {
        self.builder
            .as_ref()
            .unwrap()
            .create_subroutine_type(file, None, &[], DIFlags::Public)
    }
}
