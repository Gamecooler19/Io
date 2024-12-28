use std::path::PathBuf;
use crate::{Result, ast::ASTNode};

pub struct DocumentationGenerator {
    output_dir: PathBuf,
    template_dir: PathBuf,
}

impl DocumentationGenerator {
    pub fn new(output_dir: PathBuf) -> Result<Self> {
        let template_dir = output_dir.join("templates");
        std::fs::create_dir_all(&template_dir)?;
        
        Ok(Self {
            output_dir,
            template_dir,
        })
    }

    pub fn generate_docs(&self, ast: &ASTNode) -> Result<()> {
        // Extract documentation comments
        let docs = self.extract_documentation(ast)?;
        
        // Generate API reference
        self.generate_api_reference(&docs)?;
        
        // Generate examples
        self.generate_examples()?;
        
        Ok(())
    }
}
