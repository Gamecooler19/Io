use std::collections::HashMap;
use tower_lsp::{
    jsonrpc::Result,
    lsp_types::*,
    {Client, LanguageServer, LspService, Server},
};
use crate::{
    ast::ASTNode,
    error::IoError,
    parser::Parser,
    semantic::analyzer::SemanticAnalyzer,
    Result as IoResult,
};

pub struct IoLanguageServer {
    client: Client,
    workspace: Workspace,
    document_map: HashMap<Url, TextDocumentItem>,
    ast_cache: HashMap<Url, ASTNode>,
    semantic_cache: HashMap<Url, SemanticData>,
}

#[derive(Debug, Default)]
struct Workspace {
    root_path: Option<String>,
    config: WorkspaceConfig,
    diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Default)]
struct WorkspaceConfig {
    max_problems: i32,
    trace: TraceValue,
    format_on_save: bool,
}

#[derive(Debug)]
struct SemanticData {
    symbols: Vec<SymbolInformation>,
    references: HashMap<Position, Vec<Location>>,
    diagnostics: Vec<Diagnostic>,
}

impl IoLanguageServer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            workspace: Workspace::default(),
            document_map: HashMap::new(),
            ast_cache: HashMap::new(),
            semantic_cache: HashMap::new(),
        }
    }

    async fn analyze_document(&mut self, uri: &Url) -> IoResult<()> {
        if let Some(document) = self.document_map.get(uri) {
            // Parse document
            let mut parser = Parser::new(document.text.as_str());
            let ast = parser.parse()?;
            
            // Perform semantic analysis
            let mut analyzer = SemanticAnalyzer::new();
            let analysis_result = analyzer.check(&ast)?;
            
            // Cache results
            self.ast_cache.insert(uri.clone(), ast);
            
            // Collect symbols and references
            let mut semantic_data = SemanticData {
                symbols: Vec::new(),
                references: HashMap::new(),
                diagnostics: Vec::new(),
            };
            
            self.collect_symbols(&analysis_result, &mut semantic_data)?;
            self.semantic_cache.insert(uri.clone(), semantic_data);
            
            // Report diagnostics
            self.publish_diagnostics(uri).await?;
        }
        Ok(())
    }

    async fn publish_diagnostics(&self, uri: &Url) -> IoResult<()> {
        if let Some(semantic_data) = self.semantic_cache.get(uri) {
            self.client
                .publish_diagnostics(
                    uri.clone(),
                    semantic_data.diagnostics.clone(),
                    None,
                )
                .await;
        }
        Ok(())
    }

    fn collect_symbols(&self, ast: &ASTNode, data: &mut SemanticData) -> IoResult<()> {
        match ast {
            ASTNode::Function { name, params, return_type, .. } => {
                data.symbols.push(SymbolInformation {
                    name: name.clone(),
                    kind: SymbolKind::FUNCTION,
                    tags: None,
                    deprecated: None,
                    location: Location {
                        uri: Url::parse("file:///").unwrap(),
                        range: Range::default(),
                    },
                    container_name: None,
                });
            }
            // Add more symbol collection logic...
            _ => {}
        }
        Ok(())
    }

    async fn handle_code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let mut actions = Vec::new();
        
        if let Some(semantic_data) = self.semantic_cache.get(&params.text_document.uri) {
            for diagnostic in &semantic_data.diagnostics {
                if params.range.intersection(&diagnostic.range).is_some() {
                    // Add quick fixes based on diagnostic
                    if let Some(action) = self.create_quick_fix(diagnostic) {
                        actions.push(action);
                    }
                }
            }
        }
        
        Ok(Some(actions))
    }

    fn create_quick_fix(&self, diagnostic: &Diagnostic) -> Option<CodeActionOrCommand> {
        match diagnostic.severity {
            Some(DiagnosticSeverity::ERROR) => {
                // Create error fix
                Some(CodeActionOrCommand::CodeAction(CodeAction {
                    title: "Fix error".to_string(),
                    kind: Some(CodeActionKind::QUICKFIX),
                    diagnostics: Some(vec![diagnostic.clone()]),
                    edit: None,
                    command: None,
                    is_preferred: Some(true),
                    disabled: None,
                    data: None,
                }))
            }
            Some(DiagnosticSeverity::WARNING) => {
                // Create warning fix
                Some(CodeActionOrCommand::CodeAction(CodeAction {
                    title: "Fix warning".to_string(),
                    kind: Some(CodeActionKind::QUICKFIX),
                    diagnostics: Some(vec![diagnostic.clone()]),
                    edit: None,
                    command: None,
                    is_preferred: Some(false),
                    disabled: None,
                    data: None,
                }))
            }
            _ => None,
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for IoLanguageServer {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        if let Some(root_uri) = params.root_uri {
            self.workspace.root_path = Some(root_uri.to_string());
        }

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(true),
                    trigger_characters: Some(vec![".".to_string()]),
                    all_commit_characters: None,
                    work_done_progress_options: Default::default(),
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                signature_help_provider: Some(SignatureHelpOptions {
                    trigger_characters: Some(vec!["(".to_string(), ",".to_string()]),
                    retrigger_characters: None,
                    work_done_progress_options: Default::default(),
                }),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                document_highlight_provider: Some(OneOf::Left(true)),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                document_formatting_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                workspace_symbol_provider: Some(OneOf::Left(true)),
                implementation_provider: Some(ImplementationProviderCapability::Simple(true)),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensRegistrationOptions(
                        SemanticTokensRegistrationOptions {
                            text_document_registration_options: {
                                TextDocumentRegistrationOptions {
                                    document_selector: Some(vec![DocumentFilter {
                                        language: Some("io".to_string()),
                                        scheme: Some("file".to_string()),
                                        pattern: None,
                                    }]),
                                }
                            },
                            semantic_tokens_options: SemanticTokensOptions {
                                work_done_progress_options: Default::default(),
                                legend: SemanticTokensLegend {
                                    token_types: vec![
                                        SemanticTokenType::FUNCTION,
                                        SemanticTokenType::VARIABLE,
                                        SemanticTokenType::STRING,
                                        SemanticTokenType::NUMBER,
                                        SemanticTokenType::KEYWORD,
                                    ],
                                    token_modifiers: vec![
                                        SemanticTokenModifier::DECLARATION,
                                        SemanticTokenModifier::DEFINITION,
                                        SemanticTokenModifier::READONLY,
                                        SemanticTokenModifier::STATIC,
                                    ],
                                },
                                range: Some(true),
                                full: Some(SemanticTokensFullOptions::Delta { delta: true }),
                            },
                        },
                    ),
                ),
                ..ServerCapabilities::default()
            },
            server_info: Some(ServerInfo {
                name: "io-language-server".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client.log_message(MessageType::INFO, "Io Language Server initialized").await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        self.document_map.insert(uri.clone(), params.text_document);
        self.analyze_document(&uri).await.unwrap_or_else(|e| {
            self.client.log_message(MessageType::ERROR, format!("Analysis error: {}", e)).await;
        });
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(doc) = self.document_map.get_mut(&params.text_document.uri) {
            for change in params.content_changes {
                doc.text = change.text;
            }
            self.analyze_document(&params.text_document.uri).await.unwrap_or_else(|e| {
                self.client.log_message(MessageType::ERROR, format!("Analysis error: {}", e)).await;
            });
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        if self.workspace.config.format_on_save {
            self.format_document(&params.text_document.uri).await.unwrap_or_else(|e| {
                self.client.log_message(MessageType::ERROR, format!("Format error: {}", e)).await;
            });
        }
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let mut completions = Vec::new();
        
        if let Some(semantic_data) = self.semantic_cache.get(&params.text_document_position.text_document.uri) {
            // Add function completions
            for symbol in &semantic_data.symbols {
                if symbol.kind == SymbolKind::FUNCTION {
                    completions.push(CompletionItem {
                        label: symbol.name.clone(),
                        kind: Some(CompletionItemKind::FUNCTION),
                        detail: Some("Function".to_string()),
                        documentation: None,
                        deprecated: None,
                        preselect: None,
                        sort_text: None,
                        filter_text: None,
                        insert_text: None,
                        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                        insert_text_mode: None,
                        text_edit: None,
                        additional_text_edits: None,
                        command: None,
                        commit_characters: None,
                        data: None,
                        tags: None,
                    });
                }
            }
        }

        Ok(Some(CompletionResponse::Array(completions)))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        if let Some(semantic_data) = self.semantic_cache.get(&params.text_document_position_params.text_document.uri) {
            if let Some(symbol) = self.find_symbol_at_position(
                &semantic_data.symbols,
                params.text_document_position_params.position,
            ) {
                return Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: self.generate_hover_text(symbol),
                    }),
                    range: None,
                }));
            }
        }
        Ok(None)
    }

    async fn goto_definition(&self, params: GotoDefinitionParams) -> Result<Option<GotoDefinitionResponse>> {
        if let Some(semantic_data) = self.semantic_cache.get(&params.text_document_position_params.text_document.uri) {
            if let Some(symbol) = self.find_symbol_at_position(
                &semantic_data.symbols,
                params.text_document_position_params.position,
            ) {
                return Ok(Some(GotoDefinitionResponse::Scalar(symbol.location.clone())));
            }
        }
        Ok(None)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        if let Some(semantic_data) = self.semantic_cache.get(&params.text_document_position.text_document.uri) {
            if let Some(references) = semantic_data.references.get(&params.text_document_position.position) {
                return Ok(Some(references.clone()));
            }
        }
        Ok(None)
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        self.format_document(&params.text_document.uri).await?;
        Ok(None)
    }
}

impl IoLanguageServer {
    async fn format_document(&self, uri: &Url) -> IoResult<()> {
        if let Some(doc) = self.document_map.get(uri) {
            // Parse and format the document
            let formatted = self.format_code(&doc.text)?;
            
            // Apply formatting
            self.client.apply_edit(WorkspaceEdit {
                changes: Some(HashMap::from([(
                    uri.clone(),
                    vec![TextEdit {
                        range: Range {
                            start: Position::new(0, 0),
                            end: Position::new(u32::MAX, u32::MAX),
                        },
                        new_text: formatted,
                    }],
                )])),
                document_changes: None,
                change_annotations: None,
            }).await?;
        }
        Ok(())
    }

    fn format_code(&self, code: &str) -> IoResult<String> {
        let mut formatted = String::new();
        let mut indent_level = 0;
        let indent_str = "    ";  // 4 spaces
        
        for line in code.lines() {
            let trimmed = line.trim();
            
            // Decrease indent for closing braces
            if trimmed.starts_with('}') {
                indent_level = indent_level.saturating_sub(1);
            }
            
            // Add indentation
            if !trimmed.is_empty() {
                formatted.push_str(&indent_str.repeat(indent_level));
                formatted.push_str(trimmed);
                formatted.push('\n');
            }
            
            // Increase indent after opening braces
            if trimmed.ends_with('{') {
                indent_level += 1;
            }
            
            // Handle one-line control structures
            if trimmed.starts_with("if ") || trimmed.starts_with("for ") || trimmed.starts_with("while ") {
                if !trimmed.ends_with('{') {
                    indent_level += 1;
                }
            }
        }
        
        Ok(formatted)
    }

    fn find_symbol_at_position(
        &self,
        symbols: &[SymbolInformation],
        position: Position,
    ) -> Option<&SymbolInformation> {
        symbols.iter().find(|symbol| {
            symbol.location.range.start <= position && position <= symbol.location.range.end
        })
    }

    fn generate_hover_text(&self, symbol: &SymbolInformation) -> String {
        match symbol.kind {
            SymbolKind::FUNCTION => format!("```io\nfn {}\n```\n---\nFunction definition", symbol.name),
            SymbolKind::VARIABLE => format!("```io\nlet {}\n```\n---\nVariable declaration", symbol.name),
            _ => format!("{}: {}", symbol.kind, symbol.name),
        }
    }

    pub async fn run_server(self) -> Result<()> {
        let (service, socket) = LspService::new(|client| IoLanguageServer::new(client));
        Server::new(tokio::io::stdin(), tokio::io::stdout(), socket).serve(service).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp::lsp_types::*;
    use std::sync::Arc;

    fn setup_test_server() -> IoLanguageServer {
        let (client, _) = tower_lsp::ClientSocket::new();
        IoLanguageServer::new(client)
    }

    #[tokio::test]
    async fn test_completion() {
        let server = setup_test_server();
        
        // Add test document
        let uri = Url::parse("file:///test.io").unwrap();
        let doc = TextDocumentItem {
            uri: uri.clone(),
            language_id: "io".to_string(),
            version: 1,
            text: "fn test() {\n    print\n}".to_string(),
        };
        server.document_map.insert(uri.clone(), doc);
        
        // Test completion at 'print'
        let completion_params = CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position::new(1, 5),
            },
            context: None,
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };
        
        let completions = server.completion(completion_params).await.unwrap();
        
        match completions {
            Some(CompletionResponse::Array(items)) => {
                assert!(items.iter().any(|item| item.label == "println"));
            }
            _ => panic!("Expected completion items"),
        }
    }

    #[tokio::test]
    async fn test_hover() {
        let server = setup_test_server();
        
        // Add test document with function
        let uri = Url::parse("file:///test.io").unwrap();
        let doc = TextDocumentItem {
            uri: uri.clone(),
            language_id: "io".to_string(),
            version: 1,
            text: "fn add(a: int, b: int) -> int {\n    return a + b;\n}".to_string(),
        };
        server.document_map.insert(uri.clone(), doc);
        
        // Analyze document to populate semantic data
        server.analyze_document(&uri).await.unwrap();
        
        // Test hover over function name
        let hover_params = HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position::new(0, 3), // Position at "add"
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        
        let hover = server.hover(hover_params).await.unwrap();
        
        match hover {
            Some(hover) => {
                match hover.contents {
                    HoverContents::Markup(content) => {
                        assert!(content.value.contains("add"));
                        assert!(content.value.contains("Function"));
                    }
                    _ => panic!("Expected markup content"),
                }
            }
            None => panic!("Expected hover information"),
        }
    }

    #[tokio::test]
    async fn test_goto_definition() {
        let server = setup_test_server();
        
        // Add test document with variable definition and usage
        let uri = Url::parse("file:///test.io").unwrap();
        let doc = TextDocumentItem {
            uri: uri.clone(),
            language_id: "io".to_string(),
            version: 1,
            text: "let x = 42;\nprint(x);".to_string(),
        };
        server.document_map.insert(uri.clone(), doc);
        
        // Analyze document
        server.analyze_document(&uri).await.unwrap();
        
        // Test goto definition for variable usage
        let goto_params = GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position::new(1, 6), // Position at "x" in print(x)
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };
        
        let location = server.goto_definition(goto_params).await.unwrap();
        
        match location {
            Some(GotoDefinitionResponse::Scalar(loc)) => {
                assert_eq!(loc.uri, uri);
                assert_eq!(loc.range.start.line, 0); // Definition is on first line
            }
            _ => panic!("Expected definition location"),
        }
    }

    #[tokio::test]
    async fn test_formatting() {
        let server = setup_test_server();
        
        // Test unformatted code
        let unformatted = r#"
fn test() {
if true {
println("nested");
}
    return 42;
}
"#.trim();

        let expected = r#"fn test() {
    if true {
        println("nested");
    }
    return 42;
}
"#;

        let formatted = server.format_code(unformatted).unwrap();
        assert_eq!(formatted, expected);

        // Test formatting with multiple nested blocks
        let unformatted = r#"
fn complex() {
if condition {
for item in items {
while true {
println("deep");
}}}}"#.trim();

        let expected = r#"fn complex() {
    if condition {
        for item in items {
            while true {
                println("deep");
            }
        }
    }
}
"#;

        let formatted = server.format_code(unformatted).unwrap();
        assert_eq!(formatted, expected);

        // Test formatting with one-line conditions
        let unformatted = "if true println(\"one line\");";
        let expected = "if true\n    println(\"one line\");\n";
        
        let formatted = server.format_code(unformatted).unwrap();
        assert_eq!(formatted, expected);
    }
}
