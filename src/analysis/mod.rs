use std::collections::{HashMap, HashSet};
use crate::{ast::ASTNode, Result};

pub struct CodeAnalyzer {
    complexity_threshold: u32,
    dependencies: HashMap<String, HashSet<String>>,
    metrics: AnalysisMetrics,
}

impl CodeAnalyzer {
    pub fn new() -> Self {
        Self {
            complexity_threshold: 10,
            dependencies: HashMap::new(),
            metrics: AnalysisMetrics::default(),
        }
    }

    pub fn analyze(&mut self, ast: &ASTNode) -> Result<AnalysisReport> {
        let mut report = AnalysisReport::new();
        
        self.analyze_cyclomatic_complexity(ast, &mut report)?;
        self.analyze_dependencies(ast, &mut report)?;
        self.analyze_code_smells(ast, &mut report)?;
        
        Ok(report)
    }

    fn analyze_cyclomatic_complexity(&self, node: &ASTNode, report: &mut AnalysisReport) -> Result<u32> {
        let complexity = match node {
            ASTNode::Function { body, .. } => {
                let mut count = 1; // Base complexity
                for stmt in body {
                    count += match stmt {
                        ASTNode::If { .. } => 1,
                        ASTNode::While { .. } => 1,
                        ASTNode::Match { arms, .. } => arms.len() as u32 - 1,
                        _ => 0,
                    };
                }
                if count > self.complexity_threshold {
                    report.add_warning(format!(
                        "High cyclomatic complexity: {} (threshold: {})",
                        count,
                        self.complexity_threshold
                    ));
                }
                count
            }
            _ => 0,
        };
        Ok(complexity)
    }

    fn analyze_dependencies(&self, node: &ASTNode, report: &mut AnalysisReport) -> Result<()> {
        match node {
            ASTNode::Function { name, body, .. } => {
                let mut deps = HashSet::new();
                self.collect_dependencies(body, &mut deps);
                if deps.len() > 5 {
                    report.add_warning(format!(
                        "Function '{}' has too many dependencies: {}",
                        name,
                        deps.len()
                    ));
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn analyze_code_smells(&self, node: &ASTNode, report: &mut AnalysisReport) -> Result<()> {
        // Detect potential code smells
        match node {
            ASTNode::Function { name, body, .. } => {
                if body.len() > 50 {
                    report.add_warning(format!("Function '{}' is too long", name));
                }
                self.check_nested_blocks(body, 0, report);
            }
            _ => {}
        }
        Ok(())
    }

    fn check_nested_blocks(&self, nodes: &[ASTNode], depth: usize, report: &mut AnalysisReport) {
        if depth > 3 {
            report.add_warning("Excessive nesting detected".to_string());
        }
        for node in nodes {
            match node {
                ASTNode::Block(inner) => self.check_nested_blocks(inner, depth + 1, report),
                ASTNode::If { then_branch, else_branch, .. } => {
                    self.check_nested_blocks(then_branch, depth + 1, report);
                    if let Some(else_nodes) = else_branch {
                        self.check_nested_blocks(else_nodes, depth + 1, report);
                    }
                }
                _ => {}
            }
        }
    }
}

#[derive(Default)]
pub struct AnalysisMetrics {
    function_count: usize,
    line_count: usize,
    complexity_scores: Vec<u32>,
}

pub struct AnalysisReport {
    warnings: Vec<String>,
    metrics: AnalysisMetrics,
}

impl AnalysisReport {
    pub fn new() -> Self {
        Self {
            warnings: Vec::new(),
            metrics: AnalysisMetrics::default(),
        }
    }

    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    pub fn print_report(&self) {
        println!("\nCode Analysis Report");
        println!("===================");
        
        if !self.warnings.is_empty() {
            println!("\nWarnings:");
            for warning in &self.warnings {
                println!("- {}", warning);
            }
        }

        println!("\nMetrics:");
        println!("- Functions: {}", self.metrics.function_count);
        println!("- Lines of Code: {}", self.metrics.line_count);
        if !self.metrics.complexity_scores.is_empty() {
            let avg_complexity: f64 = self.metrics.complexity_scores.iter().sum::<u32>() as f64 
                / self.metrics.complexity_scores.len() as f64;
            println!("- Average Complexity: {:.2}", avg_complexity);
        }
    }
}
