use crate::{Result, error::IoError};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum GraphQLType {
    String,
    Int,
    Float,
    Boolean,
    ID,
    List(Box<GraphQLType>),
    Object(String),
    NonNull(Box<GraphQLType>),
}

pub struct SchemaBuilder<'ctx> {
    types: HashMap<String, GraphQLType>,
    queries: HashMap<String, FunctionSignature>,
    mutations: HashMap<String, FunctionSignature>,
    subscriptions: HashMap<String, FunctionSignature>,
    context: &'ctx inkwell::context::Context,
}

impl<'ctx> SchemaBuilder<'ctx> {
    pub fn new(context: &'ctx inkwell::context::Context) -> Self {
        Self {
            types: HashMap::new(),
            queries: HashMap::new(),
            mutations: HashMap::new(),
            subscriptions: HashMap::new(),
            context,
        }
    }

    pub fn add_type(&mut self, name: &str, gql_type: GraphQLType) -> Result<()> {
        if self.types.contains_key(name) {
            return Err(IoError::validation_error(format!("Type {} already exists", name)));
        }
        self.types.insert(name.to_string(), gql_type);
        Ok(())
    }

    pub fn add_query(&mut self, name: &str, signature: FunctionSignature) -> Result<()> {
        if self.queries.contains_key(name) {
            return Err(IoError::validation_error(format!("Query {} already exists", name)));
        }
        self.queries.insert(name.to_string(), signature);
        Ok(())
    }

    pub fn generate_schema_code(&self) -> Result<String> {
        let mut schema = String::new();
        
        // Generate type definitions
        for (name, typ) in &self.types {
            schema.push_str(&format!("type {} {{\n", name));
            if let GraphQLType::Object(fields) = typ {
                schema.push_str(fields);
            }
            schema.push_str("}\n\n");
        }

        // Generate Query type
        schema.push_str("type Query {\n");
        for (name, sig) in &self.queries {
            schema.push_str(&format!("  {}: {}\n", name, sig.to_schema_string()));
        }
        schema.push_str("}\n\n");

        // Generate Mutation type if any mutations exist
        if !self.mutations.is_empty() {
            schema.push_str("type Mutation {\n");
            for (name, sig) in &self.mutations {
                schema.push_str(&format!("  {}: {}\n", name, sig.to_schema_string()));
            }
            schema.push_str("}\n\n");
        }

        Ok(schema)
    }
}

#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub args: Vec<(String, GraphQLType)>,
    pub return_type: GraphQLType,
}

impl FunctionSignature {
    fn to_schema_string(&self) -> String {
        let args = self.args.iter()
            .map(|(name, typ)| format!("{}: {}", name, typ.to_string()))
            .collect::<Vec<_>>()
            .join(", ");
        
        if self.args.is_empty() {
            self.return_type.to_string()
        } else {
            format!("({}): {}", args, self.return_type)
        }
    }
}
