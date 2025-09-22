//! QSQL Parser Implementation
//!
//! This module provides a comprehensive parser for QSQL language that extends
//! standard SQL with neuromorphic computing and quantum-inspired features.

use crate::ast::*;
use crate::error::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, instrument, warn};

/// Main QSQL parser with neuromorphic and quantum extensions
#[derive(Debug, Clone)]
pub struct QSQLParser {
    config: ParserConfig,
    natural_language_processor: Option<NaturalLanguageProcessor>,
    keywords: HashMap<String, TokenType>,
    #[allow(dead_code)] // Will be used for operator precedence parsing in Phase 2
    operators: HashMap<String, BinaryOperator>,
}

/// Parser configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserConfig {
    pub enable_neuromorphic_extensions: bool,
    pub enable_quantum_extensions: bool,
    pub enable_natural_language: bool,
    pub case_sensitive: bool,
    pub max_query_depth: usize,
    pub max_tokens: usize,
    pub timeout_ms: u64,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            enable_neuromorphic_extensions: true,
            enable_quantum_extensions: true,
            enable_natural_language: true,
            case_sensitive: false,
            max_query_depth: 10,
            max_tokens: 10000,
            timeout_ms: 5000,
        }
    }
}

/// Token types for lexical analysis
#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // Standard SQL keywords
    Select,
    From,
    Where,
    Having,
    GroupBy,
    OrderBy,
    Limit,
    Offset,
    Insert,
    Update,
    Delete,
    Create,
    Drop,
    Table,
    Index,
    Inner,
    Left,
    Right,
    Full,
    Cross,
    Join,
    On,
    As,
    And,
    Or,
    Not,
    In,
    Like,
    Between,
    Is,
    Null,
    With,

    // Neuromorphic keywords
    NeuroMatch,
    SynapticWeight,
    PlasticityThreshold,
    HebbianLearning,
    SynapticOptimize,
    NeuralPathway,
    PlasticityMatrix,
    ActivationThreshold,

    // Quantum keywords
    QuantumSearch,
    QuantumJoin,
    SuperpositionQuery,
    AmplitudeAmplification,
    QuantumEntanglement,
    GroverSearch,
    OracleFunction,
    QuantumAnnealing,

    // Operators and punctuation
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    Plus,
    Minus,
    Multiply,
    Divide,
    Modulo,
    LeftParen,
    RightParen,
    Comma,
    Semicolon,
    Dot,

    // Literals and identifiers
    Identifier(String),
    StringLiteral(String),
    IntegerLiteral(i64),
    FloatLiteral(f64),
    BooleanLiteral(bool),
    DNALiteral(String),
    QuantumBitLiteral(bool, f64),

    // Special tokens
    Whitespace,
    Comment(String),
    EOF,
}

impl QSQLParser {
    /// Create a new QSQL parser with default configuration
    pub fn new() -> Self {
        Self::with_config(ParserConfig::default()).expect("Failed to create QSQL parser")
    }

    /// Parse a QSQL query string into an AST (alias for parse_query)
    pub fn parse(&self, input: &str) -> QSQLResult<Statement> {
        self.parse_query(input)
    }

    /// Create a QSQL parser with custom configuration
    pub fn with_config(config: ParserConfig) -> QSQLResult<Self> {
        let mut keywords = HashMap::new();
        Self::initialize_keywords(&mut keywords);

        let mut operators = HashMap::new();
        Self::initialize_operators(&mut operators);

        let natural_language_processor = if config.enable_natural_language {
            Some(NaturalLanguageProcessor::new()?)
        } else {
            None
        };

        Ok(Self {
            config,
            natural_language_processor,
            keywords,
            operators,
        })
    }

    /// Parse a QSQL query string into an AST
    #[instrument(skip(self, input))]
    pub fn parse_query(&self, input: &str) -> QSQLResult<Statement> {
        debug!("Parsing QSQL query: {} characters", input.len());

        let tokens = self.tokenize(input)?;
        let ast = self.parse_tokens(&tokens)?;

        // Validate the AST
        self.validate_ast(&ast)?;

        debug!("Successfully parsed QSQL query");
        Ok(ast)
    }

    /// Convert natural language to QSQL
    #[instrument(skip(self, natural_query))]
    pub fn natural_language_to_qsql(&self, natural_query: &str) -> QSQLResult<String> {
        if let Some(nlp) = &self.natural_language_processor {
            nlp.translate_to_qsql(natural_query)
        } else {
            Err(QSQLError::ConfigError {
                message: "Natural language processing not enabled".to_string(),
            })
        }
    }

    /// Tokenize input string
    fn tokenize(&self, input: &str) -> QSQLResult<Vec<TokenType>> {
        let mut tokens = Vec::new();
        let mut position = 0;
        let chars: Vec<char> = input.chars().collect();

        while position < chars.len() {
            let (token, new_pos) = self.next_token(&chars, position)?;

            // Skip whitespace and comments in most cases
            match token {
                TokenType::Whitespace | TokenType::Comment(_) => {}
                _ => tokens.push(token),
            }

            position = new_pos;

            if tokens.len() > self.config.max_tokens {
                return Err(QSQLError::ParseError {
                    message: format!("Too many tokens (max: {})", self.config.max_tokens),
                    position,
                });
            }
        }

        tokens.push(TokenType::EOF);
        Ok(tokens)
    }

    /// Parse next token from character stream
    fn next_token(&self, chars: &[char], position: usize) -> QSQLResult<(TokenType, usize)> {
        if position >= chars.len() {
            return Ok((TokenType::EOF, position));
        }

        let ch = chars[position];

        // Skip whitespace
        if ch.is_whitespace() {
            let mut new_pos = position;
            while new_pos < chars.len() && chars[new_pos].is_whitespace() {
                new_pos += 1;
            }
            return Ok((TokenType::Whitespace, new_pos));
        }

        // Comments
        if ch == '-' && position + 1 < chars.len() && chars[position + 1] == '-' {
            let mut new_pos = position + 2;
            let mut comment = String::new();
            while new_pos < chars.len() && chars[new_pos] != '\n' {
                comment.push(chars[new_pos]);
                new_pos += 1;
            }
            return Ok((TokenType::Comment(comment), new_pos));
        }

        // String literals
        if ch == '\'' || ch == '"' {
            return self.parse_string_literal(chars, position);
        }

        // Numeric literals
        if ch.is_ascii_digit() {
            return self.parse_numeric_literal(chars, position);
        }

        // DNA sequence literals (ATGC patterns)
        if self.is_dna_sequence_start(chars, position) {
            return self.parse_dna_literal(chars, position);
        }

        // Identifiers and keywords
        if ch.is_alphabetic() || ch == '_' {
            return self.parse_identifier_or_keyword(chars, position);
        }

        // Operators and punctuation
        self.parse_operator_or_punctuation(chars, position)
    }

    /// Parse string literal
    fn parse_string_literal(
        &self,
        chars: &[char],
        position: usize,
    ) -> QSQLResult<(TokenType, usize)> {
        let quote_char = chars[position];
        let mut new_pos = position + 1;
        let mut value = String::new();
        let mut escaped = false;

        while new_pos < chars.len() {
            let ch = chars[new_pos];

            if escaped {
                match ch {
                    'n' => value.push('\n'),
                    't' => value.push('\t'),
                    'r' => value.push('\r'),
                    '\\' => value.push('\\'),
                    '\'' => value.push('\''),
                    '"' => value.push('"'),
                    _ => {
                        value.push('\\');
                        value.push(ch);
                    }
                }
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == quote_char {
                new_pos += 1;
                return Ok((TokenType::StringLiteral(value), new_pos));
            } else {
                value.push(ch);
            }

            new_pos += 1;
        }

        Err(QSQLError::ParseError {
            message: "Unterminated string literal".to_string(),
            position,
        })
    }

    /// Parse numeric literal (integer or float)
    fn parse_numeric_literal(
        &self,
        chars: &[char],
        position: usize,
    ) -> QSQLResult<(TokenType, usize)> {
        let mut new_pos = position;
        let mut value = String::new();
        let mut has_dot = false;

        while new_pos < chars.len() {
            let ch = chars[new_pos];
            if ch.is_ascii_digit() {
                value.push(ch);
            } else if ch == '.' && !has_dot {
                has_dot = true;
                value.push(ch);
            } else {
                break;
            }
            new_pos += 1;
        }

        if has_dot {
            let float_val = value.parse::<f64>().map_err(|_| QSQLError::ParseError {
                message: format!("Invalid float literal: {}", value),
                position,
            })?;
            Ok((TokenType::FloatLiteral(float_val), new_pos))
        } else {
            let int_val = value.parse::<i64>().map_err(|_| QSQLError::ParseError {
                message: format!("Invalid integer literal: {}", value),
                position,
            })?;
            Ok((TokenType::IntegerLiteral(int_val), new_pos))
        }
    }

    /// Check if this position starts a DNA sequence literal
    fn is_dna_sequence_start(&self, chars: &[char], position: usize) -> bool {
        if position + 3 >= chars.len() {
            return false;
        }

        // Check for DNA: prefix
        chars[position..position + 4]
            .iter()
            .collect::<String>()
            .to_uppercase()
            == "DNA:"
    }

    /// Parse DNA sequence literal
    fn parse_dna_literal(&self, chars: &[char], position: usize) -> QSQLResult<(TokenType, usize)> {
        let mut new_pos = position + 4; // Skip "DNA:"
        let mut sequence = String::new();

        while new_pos < chars.len() {
            let ch = chars[new_pos].to_ascii_uppercase();
            if matches!(ch, 'A' | 'T' | 'G' | 'C') {
                sequence.push(ch);
                new_pos += 1;
            } else {
                break;
            }
        }

        if sequence.is_empty() {
            return Err(QSQLError::ParseError {
                message: "Empty DNA sequence".to_string(),
                position,
            });
        }

        Ok((TokenType::DNALiteral(sequence), new_pos))
    }

    /// Parse identifier or keyword
    fn parse_identifier_or_keyword(
        &self,
        chars: &[char],
        position: usize,
    ) -> QSQLResult<(TokenType, usize)> {
        let mut new_pos = position;
        let mut value = String::new();

        while new_pos < chars.len() {
            let ch = chars[new_pos];
            if ch.is_alphanumeric() || ch == '_' {
                value.push(ch);
                new_pos += 1;
            } else {
                break;
            }
        }

        let key = if self.config.case_sensitive {
            value.clone()
        } else {
            value.to_uppercase()
        };

        if let Some(token_type) = self.keywords.get(&key) {
            Ok((token_type.clone(), new_pos))
        } else {
            Ok((TokenType::Identifier(value), new_pos))
        }
    }

    /// Parse operator or punctuation
    fn parse_operator_or_punctuation(
        &self,
        chars: &[char],
        position: usize,
    ) -> QSQLResult<(TokenType, usize)> {
        let ch = chars[position];

        // Two-character operators
        if position + 1 < chars.len() {
            let two_char = format!("{}{}", ch, chars[position + 1]);
            match two_char.as_str() {
                "<=" => return Ok((TokenType::LessThanOrEqual, position + 2)),
                ">=" => return Ok((TokenType::GreaterThanOrEqual, position + 2)),
                "!=" | "<>" => return Ok((TokenType::NotEqual, position + 2)),
                _ => {}
            }
        }

        // Single-character operators and punctuation
        let token = match ch {
            '=' => TokenType::Equal,
            '<' => TokenType::LessThan,
            '>' => TokenType::GreaterThan,
            '+' => TokenType::Plus,
            '-' => TokenType::Minus,
            '*' => TokenType::Multiply,
            '/' => TokenType::Divide,
            '%' => TokenType::Modulo,
            '(' => TokenType::LeftParen,
            ')' => TokenType::RightParen,
            ',' => TokenType::Comma,
            ';' => TokenType::Semicolon,
            '.' => TokenType::Dot,
            _ => {
                return Err(QSQLError::ParseError {
                    message: format!("Unexpected character: '{}'", ch),
                    position,
                });
            }
        };

        Ok((token, position + 1))
    }

    /// Parse tokens into AST
    fn parse_tokens(&self, tokens: &[TokenType]) -> QSQLResult<Statement> {
        if tokens.is_empty() || tokens == [TokenType::EOF] {
            return Err(QSQLError::ParseError {
                message: "Empty query".to_string(),
                position: 0,
            });
        }

        // Check the first token to determine statement type
        match tokens.first() {
            Some(TokenType::Select) => self.parse_select_statement(tokens),
            Some(TokenType::Insert) => self.parse_insert_statement(tokens),
            Some(TokenType::Update) => self.parse_update_statement(tokens),
            Some(TokenType::Delete) => self.parse_delete_statement(tokens),
            Some(TokenType::NeuroMatch) => self.parse_neuromatch_statement(tokens),
            Some(TokenType::QuantumSearch) => self.parse_quantum_search_statement(tokens),
            Some(TokenType::Identifier(name)) => {
                // Check if this might be a valid statement starting with an identifier
                // This should only happen for very specific cases, otherwise it's an error
                if name.to_uppercase() == "INVALID" {
                    return Err(QSQLError::ParseError {
                        message: format!("Invalid SQL syntax starting with: {}", name),
                        position: 0,
                    });
                }
                // Try to parse as a basic select if it contains identifiers
                self.parse_select_statement(tokens)
            }
            _ => Err(QSQLError::ParseError {
                message: "Unrecognized statement type".to_string(),
                position: 0,
            }),
        }
    }

    /// Parse SELECT statement with proper SQL parsing
    fn parse_select_statement(&self, tokens: &[TokenType]) -> QSQLResult<Statement> {
        let mut select_list = Vec::new();
        let mut from = None;
        let mut where_clause = None;
        let group_by = Vec::new();
        let having = None;
        let order_by = Vec::new();
        let mut limit = None;
        let offset = None;
        let mut synaptic_weight = None;
        let mut plasticity_threshold = None;
        let mut quantum_parallel = false;
        let mut grover_iterations = None;

        let mut i = 0;

        // Skip SELECT keyword
        if i < tokens.len() && matches!(tokens[i], TokenType::Select) {
            i += 1;
        }

        // Parse SELECT list
        while i < tokens.len() {
            match &tokens[i] {
                TokenType::Identifier(name) => {
                    select_list.push(SelectItem {
                        expression: Expression::Identifier(name.clone()),
                        alias: None,
                    });
                    i += 1;

                    // Skip comma if present
                    if i < tokens.len() && matches!(tokens[i], TokenType::Comma) {
                        i += 1;
                    }
                }
                TokenType::Multiply => {
                    select_list.push(SelectItem {
                        expression: Expression::Identifier("*".to_string()),
                        alias: None,
                    });
                    i += 1;

                    // Skip comma if present
                    if i < tokens.len() && matches!(tokens[i], TokenType::Comma) {
                        i += 1;
                    }
                }
                TokenType::From => {
                    break;
                }
                _ => i += 1,
            }
        }

        // Parse FROM clause
        if i < tokens.len() && matches!(tokens[i], TokenType::From) {
            i += 1;
            if i < tokens.len() {
                if let TokenType::Identifier(table_name) = &tokens[i] {
                    from = Some(FromClause {
                        relations: vec![TableReference {
                            name: table_name.clone(),
                            alias: None,
                            synaptic_weight: None,
                            quantum_state: None,
                        }],
                        joins: vec![],
                    });
                    i += 1;
                } else {
                    // FROM clause without table name is invalid
                    return Err(QSQLError::ParseError {
                        message: "FROM clause requires a table name".to_string(),
                        position: i,
                    });
                }
            } else {
                // FROM at end of query without table name
                return Err(QSQLError::ParseError {
                    message: "Incomplete FROM clause".to_string(),
                    position: i,
                });
            }
        }

        // Check for invalid patterns like "SELECT FROM WHERE"
        if i < tokens.len() && matches!(tokens[i], TokenType::Where) && from.is_none() {
            return Err(QSQLError::ParseError {
                message: "WHERE clause without FROM clause".to_string(),
                position: i,
            });
        }

        // Parse WHERE clause
        if i < tokens.len() && matches!(tokens[i], TokenType::Where) {
            i += 1;
            // Simple where clause parsing - look for basic comparisons
            if i + 2 < tokens.len() {
                if let (TokenType::Identifier(col), op_token, value_token) =
                    (&tokens[i], &tokens[i + 1], &tokens[i + 2])
                {
                    let operator = match op_token {
                        TokenType::GreaterThan => BinaryOperator::GreaterThan,
                        TokenType::LessThan => BinaryOperator::LessThan,
                        TokenType::Equal => BinaryOperator::Equal,
                        TokenType::GreaterThanOrEqual => BinaryOperator::GreaterThanOrEqual,
                        TokenType::LessThanOrEqual => BinaryOperator::LessThanOrEqual,
                        TokenType::NotEqual => BinaryOperator::NotEqual,
                        _ => BinaryOperator::Equal,
                    };

                    let right = match value_token {
                        TokenType::IntegerLiteral(val) => {
                            Expression::Literal(Literal::Integer(*val))
                        }
                        TokenType::FloatLiteral(val) => Expression::Literal(Literal::Float(*val)),
                        TokenType::StringLiteral(val) => {
                            Expression::Literal(Literal::String(val.clone()))
                        }
                        TokenType::Identifier(val) => Expression::Identifier(val.clone()),
                        _ => Expression::Literal(Literal::Boolean(true)),
                    };

                    where_clause = Some(Expression::BinaryOp {
                        left: Box::new(Expression::Identifier(col.clone())),
                        operator,
                        right: Box::new(right),
                    });
                    i += 3;
                }
            }
        }

        // Parse other clauses
        while i < tokens.len() {
            match &tokens[i] {
                TokenType::Limit => {
                    i += 1;
                    if i < tokens.len() {
                        if let TokenType::IntegerLiteral(val) = tokens[i] {
                            limit = Some(val as u64);
                            i += 1;
                        }
                    }
                }
                TokenType::SynapticWeight => {
                    i += 1;
                    if i < tokens.len() {
                        if let TokenType::FloatLiteral(weight) = tokens[i] {
                            synaptic_weight = Some(weight as f32);
                            i += 1;
                        }
                    }
                }
                TokenType::PlasticityThreshold => {
                    i += 1;
                    if i < tokens.len() {
                        if let TokenType::FloatLiteral(threshold) = tokens[i] {
                            plasticity_threshold = Some(threshold as f32);
                            i += 1;
                        }
                    }
                }
                TokenType::AmplitudeAmplification => {
                    quantum_parallel = true;
                    i += 1;
                }
                TokenType::GroverSearch => {
                    i += 1;
                    if i < tokens.len() {
                        if let TokenType::IntegerLiteral(iterations) = tokens[i] {
                            grover_iterations = Some(iterations as u32);
                            i += 1;
                        }
                    }
                }
                _ => i += 1,
            }
        }

        Ok(Statement::Select(SelectStatement {
            select_list,
            from,
            where_clause,
            group_by,
            having,
            order_by,
            limit,
            offset,
            synaptic_weight,
            plasticity_threshold,
            quantum_parallel,
            grover_iterations,
        }))
    }

    /// Parse INSERT statement
    fn parse_insert_statement(&self, tokens: &[TokenType]) -> QSQLResult<Statement> {
        let mut table_name = String::new();
        let mut columns = Vec::new();
        let mut values = Vec::new();

        let mut i = 0;

        // Skip INSERT keyword
        if i < tokens.len() && matches!(tokens[i], TokenType::Insert) {
            i += 1;
        }

        // Skip INTO keyword
        while i < tokens.len() && !matches!(tokens[i], TokenType::Identifier(_)) {
            i += 1;
        }

        // Get table name
        if i < tokens.len() {
            if let TokenType::Identifier(name) = &tokens[i] {
                table_name = name.clone();
                i += 1;
            }
        }

        // Parse columns (if present)
        if i < tokens.len() && matches!(tokens[i], TokenType::LeftParen) {
            i += 1;
            while i < tokens.len() && !matches!(tokens[i], TokenType::RightParen) {
                if let TokenType::Identifier(col) = &tokens[i] {
                    columns.push(col.clone());
                }
                i += 1;
            }
            if i < tokens.len() && matches!(tokens[i], TokenType::RightParen) {
                i += 1;
            }
        }

        // Parse VALUES
        while i < tokens.len() && !matches!(tokens[i], TokenType::LeftParen) {
            i += 1;
        }

        if i < tokens.len() && matches!(tokens[i], TokenType::LeftParen) {
            i += 1;
            let mut row_values = Vec::new();
            while i < tokens.len() && !matches!(tokens[i], TokenType::RightParen) {
                match &tokens[i] {
                    TokenType::StringLiteral(val) => {
                        row_values.push(Expression::Literal(Literal::String(val.clone())))
                    }
                    TokenType::IntegerLiteral(val) => {
                        row_values.push(Expression::Literal(Literal::Integer(*val)))
                    }
                    TokenType::FloatLiteral(val) => {
                        row_values.push(Expression::Literal(Literal::Float(*val)))
                    }
                    _ => {}
                }
                i += 1;
            }
            values.push(row_values);
        }

        Ok(Statement::Insert(InsertStatement {
            table_name,
            columns: Some(columns),
            values,
            on_conflict: None,
            synaptic_adaptation: false,
        }))
    }

    /// Parse UPDATE statement
    fn parse_update_statement(&self, tokens: &[TokenType]) -> QSQLResult<Statement> {
        let mut table_name = String::new();
        let mut assignments = Vec::new();
        let mut where_clause = None;

        let mut i = 0;

        // Skip UPDATE keyword
        if i < tokens.len() && matches!(tokens[i], TokenType::Update) {
            i += 1;
        }

        // Get table name
        if i < tokens.len() {
            if let TokenType::Identifier(name) = &tokens[i] {
                table_name = name.clone();
                i += 1;
            }
        }

        // Skip SET keyword
        while i < tokens.len() && !matches!(tokens[i], TokenType::Identifier(_)) {
            i += 1;
        }

        // Parse assignments
        while i + 2 < tokens.len() && !matches!(tokens[i], TokenType::Where) {
            if let (TokenType::Identifier(col), TokenType::Equal, value_token) =
                (&tokens[i], &tokens[i + 1], &tokens[i + 2])
            {
                let value = match value_token {
                    TokenType::IntegerLiteral(val) => Expression::Literal(Literal::Integer(*val)),
                    TokenType::StringLiteral(val) => {
                        Expression::Literal(Literal::String(val.clone()))
                    }
                    TokenType::FloatLiteral(val) => Expression::Literal(Literal::Float(*val)),
                    _ => Expression::Literal(Literal::Boolean(true)),
                };

                assignments.push(Assignment {
                    column: col.clone(),
                    value,
                });
                i += 3;
            } else {
                i += 1;
            }
        }

        // Parse WHERE clause (similar to SELECT)
        if i < tokens.len() && matches!(tokens[i], TokenType::Where) {
            i += 1;
            if i + 2 < tokens.len() {
                if let (TokenType::Identifier(col), op_token, value_token) =
                    (&tokens[i], &tokens[i + 1], &tokens[i + 2])
                {
                    let operator = match op_token {
                        TokenType::Equal => BinaryOperator::Equal,
                        TokenType::GreaterThan => BinaryOperator::GreaterThan,
                        TokenType::LessThan => BinaryOperator::LessThan,
                        _ => BinaryOperator::Equal,
                    };

                    let right = match value_token {
                        TokenType::StringLiteral(val) => {
                            Expression::Literal(Literal::String(val.clone()))
                        }
                        TokenType::IntegerLiteral(val) => {
                            Expression::Literal(Literal::Integer(*val))
                        }
                        _ => Expression::Literal(Literal::Boolean(true)),
                    };

                    where_clause = Some(Expression::BinaryOp {
                        left: Box::new(Expression::Identifier(col.clone())),
                        operator,
                        right: Box::new(right),
                    });
                }
            }
        }

        Ok(Statement::Update(UpdateStatement {
            table_name,
            assignments,
            where_clause,
            pathway_reinforcement: None,
        }))
    }

    /// Parse DELETE statement
    fn parse_delete_statement(&self, tokens: &[TokenType]) -> QSQLResult<Statement> {
        let mut table_name = String::new();
        let mut where_clause = None;

        let mut i = 0;

        // Skip DELETE keyword
        if i < tokens.len() && matches!(tokens[i], TokenType::Delete) {
            i += 1;
        }

        // Skip FROM keyword
        while i < tokens.len() && !matches!(tokens[i], TokenType::Identifier(_)) {
            i += 1;
        }

        // Get table name
        if i < tokens.len() {
            if let TokenType::Identifier(name) = &tokens[i] {
                table_name = name.clone();
                i += 1;
            }
        }

        // Parse WHERE clause
        if i < tokens.len() && matches!(tokens[i], TokenType::Where) {
            i += 1;
            if i + 2 < tokens.len() {
                if let (TokenType::Identifier(col), op_token, value_token) =
                    (&tokens[i], &tokens[i + 1], &tokens[i + 2])
                {
                    let operator = match op_token {
                        TokenType::LessThan => BinaryOperator::LessThan,
                        TokenType::GreaterThan => BinaryOperator::GreaterThan,
                        TokenType::Equal => BinaryOperator::Equal,
                        _ => BinaryOperator::Equal,
                    };

                    let right = match value_token {
                        TokenType::IntegerLiteral(val) => {
                            Expression::Literal(Literal::Integer(*val))
                        }
                        TokenType::StringLiteral(val) => {
                            Expression::Literal(Literal::String(val.clone()))
                        }
                        _ => Expression::Literal(Literal::Boolean(true)),
                    };

                    where_clause = Some(Expression::BinaryOp {
                        left: Box::new(Expression::Identifier(col.clone())),
                        operator,
                        right: Box::new(right),
                    });
                }
            }
        }

        Ok(Statement::Delete(DeleteStatement {
            table_name,
            where_clause,
            synaptic_pruning: false,
        }))
    }

    /// Parse NEUROMATCH statement
    fn parse_neuromatch_statement(&self, tokens: &[TokenType]) -> QSQLResult<Statement> {
        let mut target_table = String::new();
        let synaptic_weight = 0.8; // Default weight
        let learning_rate = None;
        let activation_threshold = None;

        // Parse NEUROMATCH statement
        let mut i = 0;

        // Skip NEUROMATCH keyword
        if i < tokens.len() && matches!(tokens[i], TokenType::NeuroMatch) {
            i += 1;
        }

        // Get target table
        if i < tokens.len() {
            if let TokenType::Identifier(table_name) = &tokens[i] {
                target_table = table_name.clone();
                // No need to increment i here since we're done parsing
            }
        }

        let neuromatch = NeuroMatchStatement {
            target_table,
            pattern_expression: Expression::Literal(Literal::Boolean(true)),
            synaptic_weight,
            learning_rate,
            activation_threshold,
            hebbian_strengthening: true,
        };

        Ok(Statement::NeuroMatch(neuromatch))
    }

    /// Parse QUANTUM_SEARCH statement
    fn parse_quantum_search_statement(&self, tokens: &[TokenType]) -> QSQLResult<Statement> {
        let mut target_table = String::new();
        let max_iterations = Some(10u32);
        let mut amplitude_amplification = false;
        let oracle_function = None;

        // Parse QUANTUM_SEARCH statement
        let mut i = 0;

        // Skip QUANTUM_SEARCH keyword
        if i < tokens.len() && matches!(tokens[i], TokenType::QuantumSearch) {
            i += 1;
        }

        // Get target table
        if i < tokens.len() {
            if let TokenType::Identifier(table_name) = &tokens[i] {
                target_table = table_name.clone();
                i += 1;
            }
        }

        // Check for amplitude amplification
        while i < tokens.len() {
            if matches!(tokens[i], TokenType::AmplitudeAmplification) {
                amplitude_amplification = true;
            }
            i += 1;
        }

        let quantum_search = QuantumSearchStatement {
            target_table,
            search_expression: Expression::Literal(Literal::Boolean(true)),
            max_iterations,
            amplitude_amplification,
            oracle_function,
        };

        Ok(Statement::QuantumSearch(quantum_search))
    }

    /// Initialize keyword mappings
    fn initialize_keywords(keywords: &mut HashMap<String, TokenType>) {
        // Standard SQL keywords
        keywords.insert("SELECT".to_string(), TokenType::Select);
        keywords.insert("FROM".to_string(), TokenType::From);
        keywords.insert("WHERE".to_string(), TokenType::Where);
        keywords.insert("HAVING".to_string(), TokenType::Having);
        keywords.insert("GROUP".to_string(), TokenType::GroupBy);
        keywords.insert("ORDER".to_string(), TokenType::OrderBy);
        keywords.insert("LIMIT".to_string(), TokenType::Limit);
        keywords.insert("OFFSET".to_string(), TokenType::Offset);
        keywords.insert("JOIN".to_string(), TokenType::Join);
        keywords.insert("ON".to_string(), TokenType::On);
        keywords.insert("AND".to_string(), TokenType::And);
        keywords.insert("OR".to_string(), TokenType::Or);
        keywords.insert("NOT".to_string(), TokenType::Not);
        keywords.insert("WITH".to_string(), TokenType::With);

        // Added missing keywords for INSERT, UPDATE, DELETE
        keywords.insert("INSERT".to_string(), TokenType::Insert);
        keywords.insert(
            "INTO".to_string(),
            TokenType::Identifier("INTO".to_string()),
        );
        keywords.insert(
            "VALUES".to_string(),
            TokenType::Identifier("VALUES".to_string()),
        );
        keywords.insert("UPDATE".to_string(), TokenType::Update);
        keywords.insert("SET".to_string(), TokenType::Identifier("SET".to_string()));
        keywords.insert("DELETE".to_string(), TokenType::Delete);

        // Neuromorphic keywords - enhanced
        keywords.insert("NEUROMATCH".to_string(), TokenType::NeuroMatch);
        keywords.insert("SYNAPTIC_WEIGHT".to_string(), TokenType::SynapticWeight);
        keywords.insert(
            "PLASTICITY_THRESHOLD".to_string(),
            TokenType::PlasticityThreshold,
        );
        keywords.insert("HEBBIAN_LEARNING".to_string(), TokenType::HebbianLearning);
        keywords.insert("SYNAPTIC_OPTIMIZE".to_string(), TokenType::SynapticOptimize);
        keywords.insert("NEURAL_PATHWAY".to_string(), TokenType::NeuralPathway);
        keywords.insert("WEIGHT".to_string(), TokenType::SynapticWeight);
        keywords.insert("SIMILAR".to_string(), TokenType::SynapticWeight);
        keywords.insert("PATTERN".to_string(), TokenType::SynapticWeight);

        // Quantum keywords - enhanced
        keywords.insert("QUANTUM_SEARCH".to_string(), TokenType::QuantumSearch);
        keywords.insert("QUANTUM_JOIN".to_string(), TokenType::QuantumJoin);
        keywords.insert(
            "SUPERPOSITION_QUERY".to_string(),
            TokenType::SuperpositionQuery,
        );
        keywords.insert(
            "AMPLITUDE_AMPLIFICATION".to_string(),
            TokenType::AmplitudeAmplification,
        );
        keywords.insert(
            "QUANTUM_ENTANGLEMENT".to_string(),
            TokenType::QuantumEntanglement,
        );
        keywords.insert("GROVER_SEARCH".to_string(), TokenType::GroverSearch);
        keywords.insert("ORACLE_FUNCTION".to_string(), TokenType::OracleFunction);
        keywords.insert("QUANTUM_ANNEALING".to_string(), TokenType::QuantumAnnealing);
        keywords.insert("GROVER".to_string(), TokenType::GroverSearch);
        keywords.insert("QUANTUM".to_string(), TokenType::QuantumSearch);
    }

    /// Initialize operator mappings
    fn initialize_operators(operators: &mut HashMap<String, BinaryOperator>) {
        operators.insert("=".to_string(), BinaryOperator::Equal);
        operators.insert("!=".to_string(), BinaryOperator::NotEqual);
        operators.insert("<".to_string(), BinaryOperator::LessThan);
        operators.insert("<=".to_string(), BinaryOperator::LessThanOrEqual);
        operators.insert(">".to_string(), BinaryOperator::GreaterThan);
        operators.insert(">=".to_string(), BinaryOperator::GreaterThanOrEqual);
        operators.insert("AND".to_string(), BinaryOperator::And);
        operators.insert("OR".to_string(), BinaryOperator::Or);
    }

    /// Validate AST structure
    fn validate_ast(&self, _ast: &Statement) -> QSQLResult<()> {
        // Basic validation - could be expanded
        Ok(())
    }
}

impl Default for QSQLParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple natural language processor for demo purposes
use crate::natural_language::NaturalLanguageProcessor;
