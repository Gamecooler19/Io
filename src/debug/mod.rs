use crate::error::IoError;
use inkwell::debug_info::{DIBuilder, DICompileUnit, DIFile, DILocation, DIScope, DIType};
use inkwell::module::Module;
use std::path::Path;

pub struct DebugInfo<'ctx> {
    builder: DIBuilder<'ctx>,
    compile_unit: DICompileUnit<'ctx>,
    current_file: DIFile<'ctx>,
    current_scope: DIScope<'ctx>,
    current_line: u32,
}

impl<'ctx> DebugInfo<'ctx> {
    pub fn new(module: &Module<'ctx>, file_path: &Path) -> Result<Self, IoError> {
        let builder = module.create_debug_info_builder(true);

        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown.io");
        let directory = file_path.parent().and_then(|p| p.to_str()).unwrap_or(".");

        let compile_unit = builder.create_compile_unit(
            1, // DWARF language identifier
            file_name,
            directory,
            "io-lang compiler",
            false, // is_optimized
            "",    // compiler command line flags
            0,     // runtime_version
            "",    // split_name
            inkwell::debug_info::DWARFEmissionKind::Full,
            0,     // dwo_id
            false, // split_debug_inlining
            false, // debug_info_for_profiling
            None,  // sys_root
            None,  // sdk
        );

        let current_file = builder.create_file(file_name, directory);
        let current_scope = compile_unit.get_file().as_debug_info_scope();

        Ok(Self {
            builder,
            compile_unit,
            current_file,
            current_scope,
            current_line: 1,
        })
    }

    pub fn create_function_type(
        &self,
        name: &str,
        param_types: &[DIType<'ctx>],
        return_type: DIType<'ctx>,
    ) -> DIType<'ctx> {
        self.builder.create_subroutine_type(
            self.current_file,
            Some(return_type),
            param_types,
            inkwell::debug_info::DIFlags::Zero,
        )
    }

    pub fn set_location(&mut self, line: u32, column: u32) {
        self.current_line = line;
        // Create location and set it as current debug location
        let location = self.builder.create_debug_location(
            self.compile_unit.get_context(),
            line,
            column,
            self.current_scope,
            None,
        );
        self.builder.set_current_debug_location(location);
    }

    pub fn finalize(&self) {
        self.builder.finalize();
    }
}

pub struct SourceLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

impl SourceLocation {
    pub fn new(file: String, line: u32, column: u32) -> Self {
        Self { file, line, column }
    }
}
