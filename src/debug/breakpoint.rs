use std::collections::HashMap;
use crate::{Result, error::IoError};

#[derive(Debug, Clone, PartialEq)]
pub enum BreakpointKind {
    Line,
    Function,
    Conditional,
    Exception,
    DataWatch { variable: String },
}

#[derive(Debug, Clone)]
pub struct Breakpoint {
    id: usize,
    location: SourceLocation,
    condition: Option<String>,
    hit_count: usize,
    kind: BreakpointKind,
    enabled: bool,
    temporary: bool,
    ignore_count: usize,
    stack_depth: Option<usize>,
    expression_context: Option<ExpressionContext>,
}

#[derive(Debug, Clone)]
pub struct BreakpointManager {
    breakpoints: HashMap<usize, Breakpoint>,
    next_id: usize,
    active: bool,
}

#[derive(Debug, Clone)]
pub struct ExpressionContext {
    variables: HashMap<String, Value>,
    stack_frame: StackFrame,
}

#[derive(Debug, Clone)]
pub struct StackFrame {
    function: String,
    locals: HashMap<String, Value>,
    line: usize,
    column: usize,
}

#[derive(Debug, Clone)]
pub enum Value {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
}

impl BreakpointManager {
    pub fn new() -> Self {
        Self {
            breakpoints: HashMap::new(),
            next_id: 1,
            active: false,
        }
    }

    pub fn add_breakpoint(
        &mut self,
        file: String,
        line: usize,
        column: usize,
        condition: Option<String>,
    ) -> Result<usize> {
        let id = self.next_id;
        self.next_id += 1;

        let breakpoint = Breakpoint {
            id,
            location: SourceLocation { file, line, column },
            condition,
            hit_count: 0,
            kind: BreakpointKind::Line,
            enabled: true,
            temporary: false,
            ignore_count: 0,
            stack_depth: None,
            expression_context: None,
        };

        self.breakpoints.insert(id, breakpoint);
        Ok(id)
    }

    pub fn add_function_breakpoint(&mut self, function_name: String) -> Result<usize> {
        let id = self.next_id;
        self.next_id += 1;

        let breakpoint = Breakpoint {
            id,
            location: SourceLocation {
                file: function_name,
                line: 0,
                column: 0,
            },
            kind: BreakpointKind::Function,
            condition: None,
            hit_count: 0,
            enabled: true,
            temporary: false,
            ignore_count: 0,
            stack_depth: None,
            expression_context: None,
        };

        self.breakpoints.insert(id, breakpoint);
        Ok(id)
    }

    pub fn add_data_watchpoint(&mut self, variable: String) -> Result<usize> {
        let id = self.next_id;
        self.next_id += 1;

        let breakpoint = Breakpoint {
            id,
            location: SourceLocation {
                file: String::new(),
                line: 0,
                column: 0,
            },
            kind: BreakpointKind::DataWatch { variable: variable.clone() },
            condition: None,
            hit_count: 0,
            enabled: true,
            temporary: false,
            ignore_count: 0,
            stack_depth: None,
            expression_context: None,
        };

        self.breakpoints.insert(id, breakpoint);
        Ok(id)
    }

    pub fn add_watchpoint(
        &mut self,
        variable: String,
        condition: Option<String>,
        watch_type: WatchType,
    ) -> Result<usize> {
        let id = self.next_id;
        self.next_id += 1;

        let breakpoint = Breakpoint {
            id,
            location: SourceLocation::default(),
            kind: BreakpointKind::DataWatch {
                variable: variable.clone(),
                watch_type,
            },
            condition,
            hit_count: 0,
            enabled: true,
            temporary: false,
            ignore_count: 0,
            stack_depth: None,
            expression_context: None,
        };

        self.breakpoints.insert(id, breakpoint);
        Ok(id)
    }

    pub fn add_exception_breakpoint(
        &mut self,
        exception_type: Option<String>,
    ) -> Result<usize> {
        let id = self.next_id;
        self.next_id += 1;

        let breakpoint = Breakpoint {
            id,
            location: SourceLocation::default(),
            kind: BreakpointKind::Exception {
                exception_type: exception_type.clone(),
            },
            condition: None,
            hit_count: 0,
            enabled: true,
            temporary: false,
            ignore_count: 0,
            stack_depth: None,
            expression_context: None,
        };

        self.breakpoints.insert(id, breakpoint);
        Ok(id)
    }

    pub fn remove_breakpoint(&mut self, id: usize) -> Result<()> {
        self.breakpoints.remove(&id).ok_or_else(|| {
            IoError::runtime_error(format!("Breakpoint {} not found", id))
        })?;
        Ok(())
    }

    pub fn should_break(&mut self, location: &SourceLocation) -> bool {
        if !self.active {
            return false;
        }

        for bp in self.breakpoints.values_mut() {
            if bp.location == *location {
                bp.hit_count += 1;
                
                // Check condition if any
                if let Some(condition) = &bp.condition {
                    // Evaluate condition
                    match self.evaluate_condition(condition) {
                        Ok(true) => return true,
                        Ok(false) => continue,
                        Err(e) => {
                            eprintln!("Error evaluating breakpoint condition: {}", e);
                            continue;
                        }
                    }
                } else {
                    return true;
                }
            }
        }
        false
    }

    pub fn should_break_on_variable_change(
        &mut self,
        variable: &str,
        old_value: &Value,
        new_value: &Value
    ) -> bool {
        if !self.active {
            return false;
        }

        for bp in self.breakpoints.values_mut() {
            if let BreakpointKind::DataWatch { variable: watch_var } = &bp.kind {
                if watch_var == variable && old_value != new_value {
                    bp.hit_count += 1;
                    if bp.ignore_count > 0 {
                        bp.ignore_count -= 1;
                        continue;
                    }
                    return true;
                }
            }
        }
        false
    }

    pub fn evaluate_breakpoint(
        &mut self,
        location: &SourceLocation,
        context: ExpressionContext
    ) -> Result<bool> {
        if !self.active {
            return Ok(false);
        }

        for bp in self.breakpoints.values_mut() {
            if !bp.enabled {
                continue;
            }

            let should_break = match &bp.kind {
                BreakpointKind::Line => bp.location == *location,
                BreakpointKind::Function => {
                    context.stack_frame.function == bp.location.file
                }
                BreakpointKind::Conditional => {
                    bp.location == *location && self.evaluate_condition_with_context(
                        bp.condition.as_ref().unwrap(),
                        &context
                    )?
                }
                BreakpointKind::Exception => true, // Always break on exceptions
                BreakpointKind::DataWatch { .. } => false, // Handled separately
            };

            if should_break {
                bp.hit_count += 1;
                if bp.ignore_count > 0 {
                    bp.ignore_count -= 1;
                    continue;
                }

                if let Some(depth) = bp.stack_depth {
                    // TODO: Implement stack depth checking
                }

                if bp.temporary {
                    self.breakpoints.remove(&bp.id);
                }

                return Ok(true);
            }
        }

        Ok(false)
    }

    fn evaluate_condition(&self, condition: &str) -> Result<bool> {
        // TODO: Replace simple condition evaluation with full expression parser
        let expression = self.parse_expression(condition)?;
        self.evaluate_expression(&expression, None)
    }

    fn parse_expression(&self, condition: &str) -> Result<Expression> {
        let tokens = self.tokenize_expression(condition)?;
        let mut parser = ExpressionParser::new(tokens);
        parser.parse()
    }

    fn tokenize_expression(&self, expr: &str) -> Result<Vec<ExprToken>> {
        let mut tokens = Vec::new();
        let mut chars = expr.chars().peekable();

        while let Some(&c) = chars.peek() {
            match c {
                '0'..='9' => {
                    let mut num = String::new();
                    while let Some(&d) = chars.peek() {
                        if d.is_digit(10) || d == '.' {
                            num.push(d);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    tokens.push(ExprToken::Number(num.parse().unwrap()));
                }
                'a'..='z' | 'A'..='Z' | '_' => {
                    let mut ident = String::new();
                    while let Some(&c) = chars.peek() {
                        if c.is_alphanumeric() || c == '_' {
                            ident.push(c);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    tokens.push(ExprToken::Identifier(ident));
                }
                '=' => {
                    chars.next();
                    if chars.peek() == Some(&'=') {
                        chars.next();
                        tokens.push(ExprToken::Equal);
                    }
                }
                '<' => {
                    chars.next();
                    if chars.peek() == Some(&'=') {
                        chars.next();
                        tokens.push(ExprToken::LessEqual);
                    } else {
                        tokens.push(ExprToken::Less);
                    }
                }
                '>' => {
                    chars.next();
                    if chars.peek() == Some(&'=') {
                        chars.next();
                        tokens.push(ExprToken::GreaterEqual);
                    } else {
                        tokens.push(ExprToken::Greater);
                    }
                }
                '!' => {
                    chars.next();
                    if chars.peek() == Some(&'=') {
                        chars.next();
                        tokens.push(ExprToken::NotEqual);
                    } else {
                        tokens.push(ExprToken::Not);
                    }
                }
                '&' => {
                    chars.next();
                    if chars.peek() == Some(&'&') {
                        chars.next();
                        tokens.push(ExprToken::And);
                    }
                }
                '|' => {
                    chars.next();
                    if chars.peek() == Some(&'|') {
                        chars.next();
                        tokens.push(ExprToken::Or);
                    }
                }
                ' ' | '\t' | '\n' | '\r' => {
                    chars.next(); // Skip whitespace
                }
                _ => return Err(IoError::runtime_error(format!("Invalid character in expression: {}", c))),
            }
        }
        Ok(tokens)
    }

    fn evaluate_condition_with_context(&self, condition: &str, context: &ExpressionContext) -> Result<bool> {
        let expression = self.parse_expression(condition)?;
        self.evaluate_expression(&expression, Some(context))
    }

    fn evaluate_expression(&self, expr: &Expression, context: Option<&ExpressionContext>) -> Result<bool> {
        match expr {
            Expression::Binary { left, op, right } => {
                let left_val = self.evaluate_expression_to_value(left, context)?;
                let right_val = self.evaluate_expression_to_value(right, context)?;
                
                match op {
                    BinaryOp::Equal => Ok(left_val == right_val),
                    BinaryOp::NotEqual => Ok(left_val != right_val),
                    BinaryOp::Less => self.compare_values(&left_val, &right_val, |a, b| a < b),
                    BinaryOp::LessEqual => self.compare_values(&left_val, &right_val, |a, b| a <= b),
                    BinaryOp::Greater => self.compare_values(&left_val, &right_val, |a, b| a > b),
                    BinaryOp::GreaterEqual => self.compare_values(&left_val, &right_val, |a, b| a >= b),
                    BinaryOp::And => Ok(self.value_to_bool(&left_val)? && self.value_to_bool(&right_val)?),
                    BinaryOp::Or => Ok(self.value_to_bool(&left_val)? || self.value_to_bool(&right_val)?),
                }
            }
            Expression::Unary { op, expr } => {
                let val = self.evaluate_expression_to_value(expr, context)?;
                match op {
                    UnaryOp::Not => Ok(!self.value_to_bool(&val)?),
                }
            }
            Expression::Variable(name) => {
                if let Some(ctx) = context {
                    if let Some(val) = ctx.variables.get(name) {
                        Ok(self.value_to_bool(val)?)
                    } else {
                        Err(IoError::runtime_error(format!("Variable {} not found", name)))
                    }
                } else {
                    Err(IoError::runtime_error("No context available for variable lookup"))
                }
            }
            Expression::Literal(val) => self.value_to_bool(val),
        }
    }

    fn evaluate_expression_to_value(&self, expr: &Expression, context: Option<&ExpressionContext>) -> Result<Value> {
        match expr {
            Expression::Binary { left, op, right } => {
                let left_val = self.evaluate_expression_to_value(left, context)?;
                let right_val = self.evaluate_expression_to_value(right, context)?;
                
                match op {
                    BinaryOp::Equal | BinaryOp::NotEqual | 
                    BinaryOp::Less | BinaryOp::LessEqual |
                    BinaryOp::Greater | BinaryOp::GreaterEqual |
                    BinaryOp::And | BinaryOp::Or => {
                        Ok(Value::Boolean(self.evaluate_expression(expr, context)?))
                    }
                }
            }
            Expression::Variable(name) => {
                if let Some(ctx) = context {
                    ctx.variables.get(name).cloned()
                        .ok_or_else(|| IoError::runtime_error(format!("Variable {} not found", name)))
                } else {
                    Err(IoError::runtime_error("No context available for variable lookup"))
                }
            }
            Expression::Literal(val) => Ok(val.clone()),
            _ => Err(IoError::runtime_error("Unsupported expression type")),
        }
    }

    fn compare_values<F>(&self, left: &Value, right: &Value, compare: F) -> Result<bool>
    where
        F: Fn(f64, f64) -> bool,
    {
        match (left, right) {
            (Value::Integer(l), Value::Integer(r)) => Ok(compare(*l as f64, *r as f64)),
            (Value::Float(l), Value::Float(r)) => Ok(compare(*l, *r)),
            (Value::Integer(l), Value::Float(r)) => Ok(compare(*l as f64, *r)),
            (Value::Float(l), Value::Integer(r)) => Ok(compare(*l, *r as f64)),
            _ => Err(IoError::runtime_error("Cannot compare these value types")),
        }
    }

    fn value_to_bool(&self, value: &Value) -> Result<bool> {
        match value {
            Value::Boolean(b) => Ok(*b),
            Value::Integer(i) => Ok(*i != 0),
            Value::Float(f) => Ok(*f != 0.0),
            Value::String(s) => Ok(!s.is_empty()),
            Value::Array(arr) => Ok(!arr.is_empty()),
            Value::Object(obj) => Ok(!obj.is_empty()),
        }
    }

    pub fn get_breakpoint(&self, id: usize) -> Option<&Breakpoint> {
        self.breakpoints.get(&id)
    }

    pub fn get_breakpoint_mut(&mut self, id: usize) -> Option<&mut Breakpoint> {
        self.breakpoints.get_mut(&id)
    }

    pub fn clear_all_breakpoints(&mut self) {
        self.breakpoints.clear();
        self.next_id = 1;
    }

    pub fn update_breakpoint(
        &mut self, 
        id: usize,
        condition: Option<String>
    ) -> Result<()> {
        if let Some(bp) = self.breakpoints.get_mut(&id) {
            bp.condition = condition;
            Ok(())
        } else {
            Err(IoError::runtime_error(format!("Breakpoint {} not found", id)))
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn toggle(&mut self, active: bool) {
        self.active = active;
    }

    pub fn list_breakpoints(&self) -> Vec<&Breakpoint> {
        self.breakpoints.values().collect()
    }

    fn check_stack_depth(&self, current_depth: usize, required_depth: usize) -> bool {
        current_depth == required_depth
    }

    pub fn set_stack_depth(&mut self, id: usize, depth: Option<usize>) -> Result<()> {
        if let Some(bp) = self.breakpoints.get_mut(&id) {
            bp.stack_depth = depth;
            Ok(())
        } else {
            Err(IoError::runtime_error(format!("Breakpoint {} not found", id)))
        }
    }

    pub fn update_breakpoint_context(
        &mut self,
        id: usize,
        context: ExpressionContext,
    ) -> Result<()> {
        if let Some(bp) = self.breakpoints.get_mut(&id) {
            bp.expression_context = Some(context);
            Ok(())
        } else {
            Err(IoError::runtime_error(format!("Breakpoint {} not found", id)))
        }
    }

    pub fn get_hit_count(&self, id: usize) -> Result<usize> {
        self.breakpoints
            .get(&id)
            .map(|bp| bp.hit_count)
            .ok_or_else(|| IoError::runtime_error(format!("Breakpoint {} not found", id)))
    }

    pub fn reset_hit_count(&mut self, id: usize) -> Result<()> {
        if let Some(bp) = self.breakpoints.get_mut(&id) {
            bp.hit_count = 0;
            Ok(())
        } else {
            Err(IoError::runtime_error(format!("Breakpoint {} not found", id)))
        }
    }

    pub fn set_temporary(&mut self, id: usize, temporary: bool) -> Result<()> {
        if let Some(bp) = self.breakpoints.get_mut(&id) {
            bp.temporary = temporary;
            Ok(())
        } else {
            Err(IoError::runtime_error(format!("Breakpoint {} not found", id)))
        }
    }

    pub fn set_ignore_count(&mut self, id: usize, count: usize) -> Result<()> {
        if let Some(bp) = self.breakpoints.get_mut(&id) {
            bp.ignore_count = count;
            Ok(())
        } else {
            Err(IoError::runtime_error(format!("Breakpoint {} not found", id)))
        }
    }

    pub fn handle_exception(
        &mut self,
        exception_type: &str,
        location: &SourceLocation,
    ) -> bool {
        if !self.active {
            return false;
        }

        for bp in self.breakpoints.values_mut() {
            if let BreakpointKind::Exception { exception_type: ex_type } = &bp.kind {
                if ex_type.as_ref().map_or(true, |t| t == exception_type) {
                    bp.hit_count += 1;
                    if bp.ignore_count > 0 {
                        bp.ignore_count -= 1;
                        continue;
                    }
                    return true;
                }
            }
        }
        false
    }

    pub fn get_breakpoint_info(&self, id: usize) -> Result<BreakpointInfo> {
        let bp = self.breakpoints.get(&id)
            .ok_or_else(|| IoError::runtime_error("Breakpoint not found"))?;

        Ok(BreakpointInfo {
            id: bp.id,
            location: bp.location.clone(),
            kind: bp.kind.clone(),
            hit_count: bp.hit_count,
            enabled: bp.enabled,
            condition: bp.condition.clone(),
        })
    }

    pub fn set_breakpoint_commands(
        &mut self,
        id: usize,
        commands: Vec<DebugCommand>,
    ) -> Result<()> {
        let bp = self.breakpoints.get_mut(&id)
            .ok_or_else(|| IoError::runtime_error("Breakpoint not found"))?;
        
        bp.commands = Some(commands);
        Ok(())
    }

    pub fn execute_breakpoint_commands(
        &self,
        id: usize,
        context: &mut DebugContext,
    ) -> Result<()> {
        let bp = self.breakpoints.get(&id)
            .ok_or_else(|| IoError::runtime_error("Breakpoint not found"))?;

        if let Some(commands) = &bp.commands {
            for cmd in commands {
                self.execute_debug_command(cmd, context)?;
            }
        }
        
        Ok(())
    }

    fn execute_debug_command(
        &self,
        command: &DebugCommand,
        context: &mut DebugContext,
    ) -> Result<()> {
        match command {
            DebugCommand::Print(expr) => {
                let value = self.evaluate_expression(expr, Some(&context.expression_context))?;
                println!("{} = {:?}", expr, value);
            }
            DebugCommand::Set(var, value) => {
                let new_value = self.evaluate_expression(value, Some(&context.expression_context))?;
                context.set_variable(var, new_value)?;
            }
            DebugCommand::Call(func, args) => {
                let evaluated_args: Result<Vec<_>> = args.iter()
                    .map(|arg| self.evaluate_expression(arg, Some(&context.expression_context)))
                    .collect();
                context.call_function(func, &evaluated_args?)?;
            }
            // TODO: Add more debug commands...
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct BreakpointInfo {
    pub id: usize,
    pub location: SourceLocation,
    pub kind: BreakpointKind,
    pub hit_count: usize,
    pub enabled: bool,
    pub condition: Option<String>,
}

#[derive(Debug, Clone)]
pub enum WatchType {
    Read,
    Write,
    ReadWrite,
}

#[derive(Debug)]
pub enum DebugCommand {
    Print(String),
    Set(String, String),
    Call(String, Vec<String>),
    // TODO: Add more command types...
}

#[derive(Debug)]
pub struct DebugContext {
    pub expression_context: ExpressionContext,
    call_stack: Vec<StackFrame>,
    variables: HashMap<String, Value>,
}

impl DebugContext {
    pub fn new() -> Self {
        Self {
            expression_context: ExpressionContext {
                variables: HashMap::new(),
                stack_frame: StackFrame::default(),
            },
            call_stack: Vec::new(),
            variables: HashMap::new(),
        }
    }

    pub fn set_variable(&mut self, name: &str, value: Value) -> Result<()> {
        self.variables.insert(name.to_string(), value);
        Ok(())
    }

    pub fn call_function(&mut self, name: &str, args: &[Value]) -> Result<()> {
        // TODO: Implementation depends on runtime support
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SourceLocation {
    pub file: String,
    pub line: usize,
    pub column: usize,
}

impl std::fmt::Display for Breakpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Breakpoint {} at {}:{} (hit {} times){}",
            self.id,
            self.location.file,
            self.location.line,
            self.hit_count,
            if let Some(cond) = &self.condition {
                format!(" when {}", cond)
            } else {
                String::new()
            }
        )
    }
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Integer(_) => "integer",
            Value::Float(_) => "float",
            Value::String(_) => "string",
            Value::Boolean(_) => "boolean",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        }
    }
}

#[derive(Debug, Clone)]
enum Expression {
    Binary {
        left: Box<Expression>,
        op: BinaryOp,
        right: Box<Expression>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expression>,
    },
    Variable(String),
    Literal(Value),
}

#[derive(Debug, Clone)]
enum BinaryOp {
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
}

#[derive(Debug, Clone)]
enum UnaryOp {
    Not,
}

#[derive(Debug, Clone)]
enum ExprToken {
    Number(f64),
    Identifier(String),
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
    Not,
}

struct ExpressionParser {
    tokens: Vec<ExprToken>,
    current: usize,
}

impl ExpressionParser {
    fn new(tokens: Vec<ExprToken>) -> Self {
        Self {
            tokens,
            current: 0,
        }
    }

    fn parse(&mut self) -> Result<Expression> {
        self.parse_logical_or()
    }

    fn parse_logical_or(&mut self) -> Result<Expression> {
        let mut expr = self.parse_logical_and()?;

        while self.match_token(&[ExprToken::Or]) {
            let right = self.parse_logical_and()?;
            expr = Expression::Binary {
                left: Box::new(expr),
                op: BinaryOp::Or,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_logical_and(&mut self) -> Result<Expression> {
        let mut expr = self.parse_equality()?;

        while self.match_token(&[ExprToken::And]) {
            let right = self.parse_equality()?;
            expr = Expression::Binary {
                left: Box::new(expr),
                op: BinaryOp::And,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_equality(&mut self) -> Result<Expression> {
        let mut expr = self.parse_comparison()?;

        while self.match_token(&[ExprToken::Equal, ExprToken::NotEqual]) {
            let op = match self.previous() {
                ExprToken::Equal => BinaryOp::Equal,
                ExprToken::NotEqual => BinaryOp::NotEqual,
                _ => unreachable!(),
            };
            let right = self.parse_comparison()?;
            expr = Expression::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_comparison(&mut self) -> Result<Expression> {
        let mut expr = self.parse_primary()?;

        while self.match_token(&[
            ExprToken::Less,
            ExprToken::LessEqual,
            ExprToken::Greater,
            ExprToken::GreaterEqual,
        ]) {
            let op = match self.previous() {
                ExprToken::Less => BinaryOp::Less,
                ExprToken::LessEqual => BinaryOp::LessEqual,
                ExprToken::Greater => BinaryOp::Greater,
                ExprToken::GreaterEqual => BinaryOp::GreaterEqual,
                _ => unreachable!(),
            };
            let right = self.parse_primary()?;
            expr = Expression::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expression> {
        if self.match_token(&[ExprToken::Number]) {
            if let ExprToken::Number(n) = self.previous() {
                return Ok(Expression::Literal(Value::Float(n)));
            }
        }

        if self.match_token(&[ExprToken::Identifier]) {
            if let ExprToken::Identifier(name) = self.previous() {
                return Ok(Expression::Variable(name));
            }
        }

        Err(IoError::runtime_error("Expected expression"))
    }

    fn match_token(&mut self, types: &[ExprToken]) -> bool {
        for t in types {
            if self.check(t) {
                self.advance();
                return true;
            }
        }
        false
    }

    fn check(&self, token_type: &ExprToken) -> bool {
        if self.is_at_end() {
            false
        } else {
            std::mem::discriminant(&self.tokens[self.current]) == std::mem::discriminant(token_type)
        }
    }

    fn advance(&mut self) -> ExprToken {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len()
    }

    fn previous(&self) -> ExprToken {
        self.tokens[self.current - 1].clone()
    }
}

// Add comprehensive tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_breakpoint_creation() {
        let mut manager = BreakpointManager::new();
        let id = manager.add_breakpoint(
            "test.rs".to_string(),
            10,
            0,
            Some("x > 5".to_string()),
        ).unwrap();
        
        let bp = manager.get_breakpoint(id).unwrap();
        assert_eq!(bp.location.file, "test.rs");
        assert_eq!(bp.location.line, 10);
        assert_eq!(bp.hit_count, 0);
    }

    #[test]
    fn test_condition_evaluation() {
        let manager = BreakpointManager::new();
        let result = manager.evaluate_condition("5 > 3").unwrap();
        assert!(result);

        let result = manager.evaluate_condition("10 < 5").unwrap();
        assert!(!result);
    }

    #[test]
    fn test_expression_context() {
        let mut manager = BreakpointManager::new();
        let mut vars = HashMap::new();
        vars.insert("x".to_string(), Value::Integer(42));

        let context = ExpressionContext {
            variables: vars,
            stack_frame: StackFrame {
                function: "test_fn".to_string(),
                locals: HashMap::new(),
                line: 1,
                column: 0,
            },
        };

        let result = manager.evaluate_condition_with_context("x > 40", &context).unwrap();
        assert!(result);
    }
}
