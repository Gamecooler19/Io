use inkwell::{
    context::Context,
    debug_info::{DIBuilder, DICompileUnit, DIFile, DIFlags, DIScope, DIType},
    module::Module,
};

pub struct DebugInfo<'ctx> {
    builder: DIBuilder<'ctx>,
    compile_unit: DICompileUnit<'ctx>,
}

impl<'ctx> DebugInfo<'ctx> {
    pub fn new(module: &Module<'ctx>, filename: &str) -> Self {
        let (builder, compile_unit) = module.create_debug_info_builder(
            true,
            inkwell::debug_info::DWARFSourceLanguage::C,
            filename,
            ".",
            "CallBridge",
            false,
            "",
            0,
            "",
            inkwell::debug_info::DWARFEmissionKind::Full,
            0,
            false,
            false,
            "",
            "",
        );

        Self {
            builder,
            compile_unit,
        }
    }

    pub fn create_debug_location(
        &self,
        context: &'ctx Context,
        line: u32,
        column: u32,
        scope: DIScope<'ctx>,
        inlined_at: Option<DILocation<'ctx>>,
    ) -> DILocation<'ctx> {
        context.create_debug_location(line, column, scope, inlined_at)
    }

    pub fn create_subroutine_type(
        &self,
        file: DIFile<'ctx>,
        params: Option<DIType<'ctx>>,
        returns: &[DIType<'ctx>],
    ) -> DIType<'ctx> {
        self.builder.create_subroutine_type(file, returns, params)
    }

    pub fn create_function(
        &self,
        name: &str,
        linkage_name: Option<&str>,
        file: DIFile<'ctx>,
        line: u32,
        ty: DIType<'ctx>,
        is_local: bool,
        is_definition: bool,
        scope_line: u32,
    ) -> DIScope<'ctx> {
        self.builder.create_function(
            self.compile_unit.as_debug_info_scope(),
            name,
            linkage_name,
            file,
            line,
            ty,
            is_local,
            is_definition,
            scope_line,
            DIFlags::ZERO,
            false,
        )
    }
}
