//! QSQL Parser Implementation
//!
//! This module provides a comprehensive parser for QSQL language that extends
//! standard SQL with neuromorphic computing and quantum-inspired features.

use crate::ast::*;
use crate::error::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, instrument, warn};

/// Operator precedence levels for Pratt parsing
/// Higher values = higher precedence (binds tighter)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Precedence {
    /// Lowest precedence - starting point
    None = 0,
    /// OR logical operator
    Or = 1,
    /// AND logical operator
    And = 2,
    /// NOT unary operator
    Not = 3,
    /// Comparison operators: =, !=, <, >, <=, >=, LIKE, IN, BETWEEN
    Comparison = 4,
    /// Addition and subtraction: +, -
    Additive = 5,
    /// Multiplication, division, modulo: *, /, %
    Multiplicative = 6,
    /// Unary minus/plus
    Unary = 7,
    /// Neuromorphic operators (synaptic similarity, hebbian)
    Neuromorphic = 8,
    /// Quantum operators (entanglement, superposition)
    Quantum = 9,
    /// Function calls and parenthesized expressions
    Call = 10,
}

impl Precedence {
    /// Get the next higher precedence level
    pub fn next(self) -> Self {
        match self {
            Precedence::None => Precedence::Or,
            Precedence::Or => Precedence::And,
            Precedence::And => Precedence::Not,
            Precedence::Not => Precedence::Comparison,
            Precedence::Comparison => Precedence::Additive,
            Precedence::Additive => Precedence::Multiplicative,
            Precedence::Multiplicative => Precedence::Unary,
            Precedence::Unary => Precedence::Neuromorphic,
            Precedence::Neuromorphic => Precedence::Quantum,
            Precedence::Quantum => Precedence::Call,
            Precedence::Call => Precedence::Call, // Max level
        }
    }
}

/// Operator information for Pratt parsing
#[derive(Debug, Clone)]
pub struct OperatorInfo {
    /// The binary operator type
    pub operator: BinaryOperator,
    /// Precedence level for this operator
    pub precedence: Precedence,
    /// Whether this operator is right-associative (false = left-associative)
    pub right_associative: bool,
}

/// Main QSQL parser with neuromorphic and quantum extensions
#[derive(Debug, Clone)]
pub struct QSQLParser {
    config: ParserConfig,
    natural_language_processor: Option<NaturalLanguageProcessor>,
    keywords: HashMap<String, TokenType>,
    /// Operator precedence map for Pratt parsing
    operators: HashMap<String, OperatorInfo>,
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
    Alter,
    Truncate,
    Table,
    Index,
    Add,
    Column,
    Modify,
    If,
    Exists,
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
    Recursive,
    Distinct,
    Extract,

    // CASE expression keywords
    Case,
    When,
    Then,
    Else,
    End,

    // Data type keywords
    Serial,
    BigSerial,
    SmallSerial,
    AutoIncrement,
    Primary,
    Key,
    Unique,
    References,
    Default,
    Generated,
    Always,
    Identity,

    // Date/Time keywords
    Interval,

    // Neuromorphic keywords
    NeuroMatch,
    SynapticWeight,
    PlasticityThreshold,
    HebbianLearning,
    SynapticOptimize,
    NeuralPathway,
    PlasticityMatrix,
    ActivationThreshold,
    Learn,
    Pattern,
    Adapt,
    Weights,
    Algorithm,
    Epochs,
    LearningRate,
    Rule,
    TrainingData,
    Features,

    // Quantum keywords
    QuantumSearch,
    QuantumJoin,
    SuperpositionQuery,
    AmplitudeAmplification,
    QuantumEntanglement,
    GroverSearch,
    OracleFunction,
    QuantumAnnealing,
    Entangle,
    Superposition,
    Coherence,
    Using,
    QuantumState,

    // Transaction control keywords
    Begin,
    Start,
    Transaction,
    Commit,
    Rollback,
    Savepoint,
    Release,
    To,

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
            Some(TokenType::With) => self.parse_select_statement(tokens),
            Some(TokenType::Select) => self.parse_select_statement(tokens),
            Some(TokenType::Insert) => self.parse_insert_statement(tokens),
            Some(TokenType::Update) => self.parse_update_statement(tokens),
            Some(TokenType::Delete) => self.parse_delete_statement(tokens),
            Some(TokenType::Create) => self.parse_create_statement(tokens),
            Some(TokenType::Drop) => self.parse_drop_statement(tokens),
            Some(TokenType::Alter) => self.parse_alter_table_statement(tokens),
            Some(TokenType::Truncate) => self.parse_truncate_table_statement(tokens),
            Some(TokenType::NeuroMatch) => self.parse_neuromatch_statement(tokens),
            Some(TokenType::QuantumSearch) => self.parse_quantum_search_statement(tokens),
            Some(TokenType::Learn) => self.parse_learn_pattern_statement(tokens),
            Some(TokenType::Adapt) => self.parse_adapt_weights_statement(tokens),
            Some(TokenType::QuantumJoin) => self.parse_quantum_join_statement(tokens),
            // Transaction control statements
            Some(TokenType::Begin) => self.parse_begin_transaction(tokens),
            Some(TokenType::Start) => self.parse_start_transaction(tokens),
            Some(TokenType::Commit) => self.parse_commit(tokens),
            Some(TokenType::Rollback) => self.parse_rollback(tokens),
            Some(TokenType::Savepoint) => self.parse_savepoint(tokens),
            Some(TokenType::Release) => self.parse_release_savepoint(tokens),
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
        let mut group_by = Vec::new();
        let mut having = None;
        let mut order_by = Vec::new();
        let mut limit = None;
        let offset = None;
        let synaptic_weight = None;
        let plasticity_threshold = None;
        let quantum_parallel = false;
        let grover_iterations = None;
        let mut with_clause = None;

        let mut i = 0;

        // Parse optional WITH clause
        if i < tokens.len() && matches!(tokens[i], TokenType::With) {
            with_clause = Some(self.parse_with_clause(tokens, &mut i)?);
        }

        // Skip SELECT keyword
        if i < tokens.len() && matches!(tokens[i], TokenType::Select) {
            i += 1;
        }

        // Skip optional DISTINCT
        if i < tokens.len() && matches!(tokens[i], TokenType::Distinct) {
            i += 1;
        }

        // Parse SELECT list
        loop {
            if i >= tokens.len() || matches!(tokens[i], TokenType::From | TokenType::EOF) {
                break;
            }

            match &tokens[i] {
                TokenType::Identifier(name) => {
                    // Build full qualified name (e.g., u.name, table.column)
                    let mut full_name = name.clone();
                    i += 1;

                    // Check for qualified name (e.g., u.id, table.column)
                    while i + 1 < tokens.len() && matches!(tokens[i], TokenType::Dot) {
                        if let TokenType::Identifier(next_part) = &tokens[i + 1] {
                            full_name.push('.');
                            full_name.push_str(next_part);
                            i += 2; // consume '.' and identifier
                        } else {
                            break;
                        }
                    }

                    // Check if this is a function call (next token is '(')
                    if i < tokens.len() && matches!(tokens[i], TokenType::LeftParen) {
                        let expr = self.parse_function_call(tokens, &mut i, full_name)?;

                        // Check for optional AS alias
                        let alias = if i < tokens.len() && matches!(tokens[i], TokenType::As) {
                            i += 1;
                            if i < tokens.len() {
                                if let TokenType::Identifier(alias_name) = &tokens[i] {
                                    i += 1;
                                    Some(alias_name.clone())
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        select_list.push(SelectItem::Expression { expr, alias });
                    } else {
                        // Check for optional AS alias
                        let alias = if i < tokens.len() && matches!(tokens[i], TokenType::As) {
                            i += 1;
                            if i < tokens.len() {
                                if let TokenType::Identifier(alias_name) = &tokens[i] {
                                    i += 1;
                                    Some(alias_name.clone())
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        select_list.push(SelectItem::Expression {
                            expr: Expression::Identifier(full_name),
                            alias,
                        });
                    }
                }
                TokenType::Multiply => {
                    select_list.push(SelectItem::Wildcard);
                    i += 1;
                }
                TokenType::Case => {
                    // Parse CASE expression
                    let expr = self.parse_case_expression(tokens, &mut i)?;

                    // Check for optional AS alias
                    let alias = if i < tokens.len() && matches!(tokens[i], TokenType::As) {
                        i += 1;
                        if i < tokens.len() {
                            if let TokenType::Identifier(alias_name) = &tokens[i] {
                                i += 1;
                                Some(alias_name.clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    select_list.push(SelectItem::Expression { expr, alias });
                }
                TokenType::Extract => {
                    // Parse EXTRACT expression
                    let expr = self.parse_extract_expression(tokens, &mut i)?;

                    // Check for optional AS alias
                    let alias = if i < tokens.len() && matches!(tokens[i], TokenType::As) {
                        i += 1;
                        if i < tokens.len() {
                            if let TokenType::Identifier(alias_name) = &tokens[i] {
                                i += 1;
                                Some(alias_name.clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    select_list.push(SelectItem::Expression { expr, alias });
                }
                TokenType::Comma => {
                    i += 1; // Skip comma and continue
                }
                _ => break,
            }
        }

        // Validate that we have at least one column in SELECT list
        if select_list.is_empty() {
            return Err(QSQLError::ParseError {
                message: "SELECT statement must specify at least one column".to_string(),
                position: i,
            });
        }

        // Parse FROM clause
        if i < tokens.len() && matches!(tokens[i], TokenType::From) {
            i += 1;
            if i < tokens.len() {
                // Check if this is a derived table (subquery) or a regular table
                let first_relation = if matches!(tokens[i], TokenType::LeftParen) {
                    // This is a derived table (subquery)
                    self.parse_derived_table(tokens, &mut i)?
                } else if let TokenType::Identifier(table_name) = &tokens[i] {
                    let first_table_name = table_name.clone();
                    i += 1;

                    // Check for table alias (AS or direct identifier)
                    let first_alias = self.parse_table_alias(tokens, &mut i);

                    TableReference {
                        name: first_table_name,
                        alias: first_alias,
                        synaptic_weight: None,
                        quantum_state: None,
                        subquery: None,
                    }
                } else {
                    return Err(QSQLError::ParseError {
                        message: "Expected table name or subquery in FROM clause".to_string(),
                        position: i,
                    });
                };

                // Parse JOINs
                let joins = self.parse_join_clauses(tokens, &mut i)?;

                from = Some(FromClause {
                    relations: vec![first_relation],
                    joins,
                });
            }
        }

        // Parse WHERE clause
        if i < tokens.len() && matches!(tokens[i], TokenType::Where) {
            i += 1;
            where_clause = Some(self.parse_expression(tokens, &mut i)?);
        }

        // Parse GROUP BY clause
        if i + 1 < tokens.len() && matches!(tokens[i], TokenType::GroupBy) {
            i += 2; // Skip "GROUP BY"
            loop {
                if i >= tokens.len()
                    || matches!(
                        tokens[i],
                        TokenType::Having | TokenType::OrderBy | TokenType::Limit | TokenType::EOF
                    )
                {
                    break;
                }
                group_by.push(self.parse_expression(tokens, &mut i)?);
                if i < tokens.len() && matches!(tokens[i], TokenType::Comma) {
                    i += 1;
                } else {
                    break;
                }
            }
        }

        // Parse HAVING clause
        if i < tokens.len() && matches!(tokens[i], TokenType::Having) {
            i += 1;
            having = Some(self.parse_expression(tokens, &mut i)?);
        }

        // Parse ORDER BY clause
        if i + 1 < tokens.len() && matches!(tokens[i], TokenType::OrderBy) {
            i += 2; // Skip "ORDER BY"
            loop {
                if i >= tokens.len() || matches!(tokens[i], TokenType::Limit | TokenType::EOF) {
                    break;
                }
                let expr = self.parse_expression(tokens, &mut i)?;
                order_by.push(OrderByItem {
                    expression: expr,
                    ascending: true, // Default to ASC
                });
                if i < tokens.len() && matches!(tokens[i], TokenType::Comma) {
                    i += 1;
                } else {
                    break;
                }
            }
        }

        // Parse LIMIT clause
        if i < tokens.len() && matches!(tokens[i], TokenType::Limit) {
            i += 1;
            if i < tokens.len() {
                if let TokenType::IntegerLiteral(n) = tokens[i] {
                    limit = Some(n as u64);
                }
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
            with_clause,
        }))
    }

    /// Parse SELECT statement starting at a specific position (for subqueries)
    /// This method is similar to parse_select_statement but accepts and updates an index.
    fn parse_select_statement_at(
        &self,
        tokens: &[TokenType],
        i: &mut usize,
    ) -> QSQLResult<SelectStatement> {
        let mut select_list = Vec::new();
        let mut from = None;
        let mut where_clause = None;
        let mut group_by = Vec::new();
        let mut having = None;
        let mut order_by = Vec::new();
        let mut limit = None;
        let offset = None;
        let synaptic_weight = None;
        let plasticity_threshold = None;
        let quantum_parallel = false;
        let grover_iterations = None;

        // Skip SELECT keyword
        if *i < tokens.len() && matches!(tokens[*i], TokenType::Select) {
            *i += 1;
        }

        // Skip optional DISTINCT
        if *i < tokens.len() && matches!(tokens[*i], TokenType::Distinct) {
            *i += 1;
        }

        // Parse SELECT list
        loop {
            if *i >= tokens.len()
                || matches!(
                    tokens[*i],
                    TokenType::From | TokenType::EOF | TokenType::RightParen
                )
            {
                break;
            }

            match &tokens[*i] {
                TokenType::Identifier(name) => {
                    // Build full qualified name (e.g., u.name, table.column)
                    let mut full_name = name.clone();
                    *i += 1;

                    // Check for qualified name (e.g., u.id, table.column)
                    while *i + 1 < tokens.len() && matches!(tokens[*i], TokenType::Dot) {
                        if let TokenType::Identifier(next_part) = &tokens[*i + 1] {
                            full_name.push('.');
                            full_name.push_str(next_part);
                            *i += 2; // consume '.' and identifier
                        } else {
                            break;
                        }
                    }

                    // Check if this is a function call (next token is '(')
                    if *i < tokens.len() && matches!(tokens[*i], TokenType::LeftParen) {
                        let expr = self.parse_function_call(tokens, i, full_name)?;

                        // Check for optional AS alias
                        let alias = if *i < tokens.len() && matches!(tokens[*i], TokenType::As) {
                            *i += 1;
                            if *i < tokens.len() {
                                if let TokenType::Identifier(alias_name) = &tokens[*i] {
                                    *i += 1;
                                    Some(alias_name.clone())
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        select_list.push(SelectItem::Expression { expr, alias });
                    } else {
                        // Check for optional AS alias
                        let alias = if *i < tokens.len() && matches!(tokens[*i], TokenType::As) {
                            *i += 1;
                            if *i < tokens.len() {
                                if let TokenType::Identifier(alias_name) = &tokens[*i] {
                                    *i += 1;
                                    Some(alias_name.clone())
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        select_list.push(SelectItem::Expression {
                            expr: Expression::Identifier(full_name),
                            alias,
                        });
                    }
                }
                TokenType::Multiply => {
                    select_list.push(SelectItem::Wildcard);
                    *i += 1;
                }
                TokenType::Case => {
                    // Parse CASE expression
                    let expr = self.parse_case_expression(tokens, i)?;

                    // Check for optional AS alias
                    let alias = if *i < tokens.len() && matches!(tokens[*i], TokenType::As) {
                        *i += 1;
                        if *i < tokens.len() {
                            if let TokenType::Identifier(alias_name) = &tokens[*i] {
                                *i += 1;
                                Some(alias_name.clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    select_list.push(SelectItem::Expression { expr, alias });
                }
                TokenType::Extract => {
                    // Parse EXTRACT expression
                    let expr = self.parse_extract_expression(tokens, i)?;

                    // Check for optional AS alias
                    let alias = if *i < tokens.len() && matches!(tokens[*i], TokenType::As) {
                        *i += 1;
                        if *i < tokens.len() {
                            if let TokenType::Identifier(alias_name) = &tokens[*i] {
                                *i += 1;
                                Some(alias_name.clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    select_list.push(SelectItem::Expression { expr, alias });
                }
                TokenType::Comma => {
                    *i += 1; // Skip comma and continue
                }
                _ => break,
            }
        }

        // Validate that we have at least one column in SELECT list
        if select_list.is_empty() {
            return Err(QSQLError::ParseError {
                message: "SELECT statement must specify at least one column".to_string(),
                position: *i,
            });
        }

        // Parse FROM clause
        if *i < tokens.len() && matches!(tokens[*i], TokenType::From) {
            *i += 1;
            if *i < tokens.len() {
                // Check if this is a derived table (subquery) or a regular table
                let first_relation = if matches!(tokens[*i], TokenType::LeftParen) {
                    // This is a derived table (subquery)
                    self.parse_derived_table(tokens, i)?
                } else if let TokenType::Identifier(table_name) = &tokens[*i] {
                    let first_table_name = table_name.clone();
                    *i += 1;

                    // Check for table alias (AS or direct identifier)
                    let first_alias = self.parse_table_alias(tokens, i);

                    TableReference {
                        name: first_table_name,
                        alias: first_alias,
                        synaptic_weight: None,
                        quantum_state: None,
                        subquery: None,
                    }
                } else {
                    return Err(QSQLError::ParseError {
                        message: "Expected table name or subquery in FROM clause".to_string(),
                        position: *i,
                    });
                };

                // Parse JOINs
                let joins = self.parse_join_clauses(tokens, i)?;

                from = Some(FromClause {
                    relations: vec![first_relation],
                    joins,
                });
            }
        }

        // Parse WHERE clause (but stop before RightParen for subqueries)
        if *i < tokens.len() && matches!(tokens[*i], TokenType::Where) {
            *i += 1;
            where_clause = Some(self.parse_subquery_where_expression(tokens, i)?);
        }

        // Parse GROUP BY clause
        if *i + 1 < tokens.len() && matches!(tokens[*i], TokenType::GroupBy) {
            *i += 2; // Skip "GROUP BY"
            loop {
                if *i >= tokens.len()
                    || matches!(
                        tokens[*i],
                        TokenType::Having
                            | TokenType::OrderBy
                            | TokenType::Limit
                            | TokenType::EOF
                            | TokenType::RightParen
                    )
                {
                    break;
                }
                group_by.push(self.parse_expression(tokens, i)?);
                if *i < tokens.len() && matches!(tokens[*i], TokenType::Comma) {
                    *i += 1;
                } else {
                    break;
                }
            }
        }

        // Parse HAVING clause
        if *i < tokens.len() && matches!(tokens[*i], TokenType::Having) {
            *i += 1;
            having = Some(self.parse_subquery_where_expression(tokens, i)?);
        }

        // Parse ORDER BY clause
        if *i + 1 < tokens.len() && matches!(tokens[*i], TokenType::OrderBy) {
            *i += 2; // Skip "ORDER BY"
            loop {
                if *i >= tokens.len()
                    || matches!(
                        tokens[*i],
                        TokenType::Limit | TokenType::EOF | TokenType::RightParen
                    )
                {
                    break;
                }
                let expr = self.parse_expression(tokens, i)?;
                order_by.push(OrderByItem {
                    expression: expr,
                    ascending: true, // Default to ASC
                });
                if *i < tokens.len() && matches!(tokens[*i], TokenType::Comma) {
                    *i += 1;
                } else {
                    break;
                }
            }
        }

        // Parse LIMIT clause
        if *i < tokens.len() && matches!(tokens[*i], TokenType::Limit) {
            *i += 1;
            if *i < tokens.len() {
                if let TokenType::IntegerLiteral(n) = tokens[*i] {
                    limit = Some(n as u64);
                    *i += 1;
                }
            }
        }

        Ok(SelectStatement {
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
            with_clause: None, // Subqueries don't support WITH clauses (for now)
        })
    }

    /// Parse a WHERE expression for subqueries (stops at RightParen)
    fn parse_subquery_where_expression(
        &self,
        tokens: &[TokenType],
        i: &mut usize,
    ) -> QSQLResult<Expression> {
        self.parse_subquery_expression_with_precedence(tokens, i, Precedence::None)
    }

    /// Parse expression for subqueries, stopping at RightParen
    fn parse_subquery_expression_with_precedence(
        &self,
        tokens: &[TokenType],
        i: &mut usize,
        min_precedence: Precedence,
    ) -> QSQLResult<Expression> {
        // First, parse the left-hand side (prefix expression)
        let mut left = self.parse_prefix_expression(tokens, i)?;

        // Then, handle infix operators using precedence climbing
        while *i < tokens.len() {
            // Stop at RightParen for subqueries
            if matches!(tokens[*i], TokenType::RightParen) {
                break;
            }

            // Check for IS NULL or IS NOT NULL
            if matches!(tokens[*i], TokenType::Is) {
                *i += 1; // consume IS
                let negated = if *i < tokens.len() && matches!(tokens[*i], TokenType::Not) {
                    *i += 1; // consume NOT
                    true
                } else {
                    false
                };

                // Expect NULL
                if *i < tokens.len() && matches!(tokens[*i], TokenType::Null) {
                    *i += 1; // consume NULL
                    left = Expression::IsNull {
                        expr: Box::new(left),
                        negated,
                    };
                    continue;
                } else {
                    return Err(QSQLError::ParseError {
                        message: "Expected NULL after IS".to_string(),
                        position: *i,
                    });
                }
            }

            // Check for NOT IN (two-token sequence)
            if matches!(tokens[*i], TokenType::Not)
                && *i + 1 < tokens.len()
                && matches!(tokens[*i + 1], TokenType::In)
            {
                *i += 2; // consume NOT IN
                left = self.parse_in_list(tokens, i, left, true)?;
                continue;
            }

            // Check for IN operator - special handling for IN (list)
            if matches!(tokens[*i], TokenType::In) {
                *i += 1; // consume IN
                left = self.parse_in_list(tokens, i, left, false)?;
                continue;
            }

            // Get operator info for current token
            let op_info = match self.get_operator_info(&tokens[*i]) {
                Some(info) => info,
                None => break, // Not an operator, stop parsing
            };

            // If operator precedence is too low, stop
            if op_info.precedence < min_precedence {
                break;
            }

            // Consume the operator
            *i += 1;

            // Parse right-hand side with appropriate precedence
            let next_precedence = if op_info.right_associative {
                op_info.precedence
            } else {
                op_info.precedence.next()
            };

            // Stop at RightParen for subqueries
            if *i < tokens.len() && matches!(tokens[*i], TokenType::RightParen) {
                return Err(QSQLError::ParseError {
                    message: "Unexpected ) in expression".to_string(),
                    position: *i,
                });
            }

            let right =
                self.parse_subquery_expression_with_precedence(tokens, i, next_precedence)?;

            left = Expression::BinaryOp {
                left: Box::new(left),
                operator: op_info.operator,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// Parse INSERT statement
    fn parse_insert_statement(&self, tokens: &[TokenType]) -> QSQLResult<Statement> {
        let mut i = 0;

        // Skip INSERT keyword
        if i < tokens.len() && matches!(tokens[i], TokenType::Insert) {
            i += 1;
        }

        // Skip INTO keyword (optional in some SQL dialects)
        if i < tokens.len() {
            if let TokenType::Identifier(ref name) = tokens[i] {
                if name.to_uppercase() == "INTO" {
                    i += 1;
                }
            }
        }

        // Parse table name
        let table_name = if i < tokens.len() {
            if let TokenType::Identifier(name) = &tokens[i] {
                i += 1;
                name.clone()
            } else {
                return Err(QSQLError::ParseError {
                    message: "Expected table name after INSERT".to_string(),
                    position: i,
                });
            }
        } else {
            return Err(QSQLError::ParseError {
                message: "Incomplete INSERT statement".to_string(),
                position: i,
            });
        };

        // Parse column list (optional)
        let mut columns = None;
        if i < tokens.len() && matches!(tokens[i], TokenType::LeftParen) {
            i += 1; // Skip '('
            let mut col_list = Vec::new();

            loop {
                if i >= tokens.len() {
                    return Err(QSQLError::ParseError {
                        message: "Unclosed column list".to_string(),
                        position: i,
                    });
                }

                if matches!(tokens[i], TokenType::RightParen) {
                    i += 1; // Skip ')'
                    break;
                }

                if let TokenType::Identifier(col_name) = &tokens[i] {
                    col_list.push(col_name.clone());
                    i += 1;

                    if i < tokens.len() && matches!(tokens[i], TokenType::Comma) {
                        i += 1; // Skip ','
                    }
                } else {
                    return Err(QSQLError::ParseError {
                        message: "Expected column name".to_string(),
                        position: i,
                    });
                }
            }

            columns = Some(col_list);
        }

        // Parse VALUES clause
        if i < tokens.len() {
            if let TokenType::Identifier(ref name) = tokens[i] {
                if name.to_uppercase() == "VALUES" {
                    i += 1;
                } else {
                    return Err(QSQLError::ParseError {
                        message: "Expected VALUES clause".to_string(),
                        position: i,
                    });
                }
            } else {
                return Err(QSQLError::ParseError {
                    message: "Expected VALUES clause".to_string(),
                    position: i,
                });
            }
        } else {
            return Err(QSQLError::ParseError {
                message: "Expected VALUES clause".to_string(),
                position: i,
            });
        }

        let mut values = Vec::new();

        // Parse value lists
        loop {
            if i >= tokens.len() || matches!(tokens[i], TokenType::EOF) {
                break;
            }

            if matches!(tokens[i], TokenType::LeftParen) {
                i += 1; // Skip '('
                let mut value_list = Vec::new();

                loop {
                    if i >= tokens.len() {
                        return Err(QSQLError::ParseError {
                            message: "Unclosed value list".to_string(),
                            position: i,
                        });
                    }

                    if matches!(tokens[i], TokenType::RightParen) {
                        i += 1; // Skip ')'
                        break;
                    }

                    value_list.push(self.parse_expression(tokens, &mut i)?);

                    if i < tokens.len() && matches!(tokens[i], TokenType::Comma) {
                        i += 1; // Skip ','
                    }
                }

                values.push(value_list);

                if i < tokens.len() && matches!(tokens[i], TokenType::Comma) {
                    i += 1; // Skip ',' between value lists
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        Ok(Statement::Insert(InsertStatement {
            table_name,
            columns,
            values,
            on_conflict: None,
            synaptic_adaptation: false,
        }))
    }

    /// Parse UPDATE statement
    fn parse_update_statement(&self, tokens: &[TokenType]) -> QSQLResult<Statement> {
        let mut i = 0;

        // Skip UPDATE keyword
        if i < tokens.len() && matches!(tokens[i], TokenType::Update) {
            i += 1;
        }

        // Parse table name
        let table_name = if i < tokens.len() {
            if let TokenType::Identifier(name) = &tokens[i] {
                i += 1;
                name.clone()
            } else {
                return Err(QSQLError::ParseError {
                    message: "Expected table name after UPDATE".to_string(),
                    position: i,
                });
            }
        } else {
            return Err(QSQLError::ParseError {
                message: "Incomplete UPDATE statement".to_string(),
                position: i,
            });
        };

        // Parse SET clause
        if i < tokens.len() {
            if let TokenType::Identifier(ref name) = tokens[i] {
                if name.to_uppercase() == "SET" {
                    i += 1;
                } else {
                    return Err(QSQLError::ParseError {
                        message: "Expected SET clause".to_string(),
                        position: i,
                    });
                }
            } else {
                return Err(QSQLError::ParseError {
                    message: "Expected SET clause".to_string(),
                    position: i,
                });
            }
        } else {
            return Err(QSQLError::ParseError {
                message: "Expected SET clause".to_string(),
                position: i,
            });
        }

        let mut assignments = Vec::new();

        // Parse column = value assignments
        loop {
            if i >= tokens.len() || matches!(tokens[i], TokenType::Where | TokenType::EOF) {
                break;
            }

            if let TokenType::Identifier(col_name) = &tokens[i] {
                i += 1;

                if i < tokens.len() && matches!(tokens[i], TokenType::Equal) {
                    i += 1;
                    let value = self.parse_expression(tokens, &mut i)?;
                    assignments.push(Assignment {
                        column: col_name.clone(),
                        value,
                    });

                    if i < tokens.len() && matches!(tokens[i], TokenType::Comma) {
                        i += 1; // Skip ','
                    } else {
                        break;
                    }
                } else {
                    return Err(QSQLError::ParseError {
                        message: "Expected '=' after column name".to_string(),
                        position: i,
                    });
                }
            } else {
                return Err(QSQLError::ParseError {
                    message: "Expected column name in SET clause".to_string(),
                    position: i,
                });
            }
        }

        // Parse WHERE clause
        let mut where_clause = None;
        if i < tokens.len() && matches!(tokens[i], TokenType::Where) {
            i += 1;
            where_clause = Some(self.parse_expression(tokens, &mut i)?);
        }

        Ok(Statement::Update(UpdateStatement {
            table_name,
            assignments,
            where_clause,
            plasticity_adaptation: None,
        }))
    }

    /// Parse DELETE statement
    fn parse_delete_statement(&self, tokens: &[TokenType]) -> QSQLResult<Statement> {
        let mut i = 0;

        // Skip DELETE keyword
        if i < tokens.len() && matches!(tokens[i], TokenType::Delete) {
            i += 1;
        }

        // Skip FROM keyword
        if i < tokens.len() && matches!(tokens[i], TokenType::From) {
            i += 1;
        } else {
            return Err(QSQLError::ParseError {
                message: "Expected FROM after DELETE".to_string(),
                position: i,
            });
        }

        // Parse table name
        let table_name = if i < tokens.len() {
            if let TokenType::Identifier(name) = &tokens[i] {
                i += 1;
                name.clone()
            } else {
                return Err(QSQLError::ParseError {
                    message: "Expected table name after FROM".to_string(),
                    position: i,
                });
            }
        } else {
            return Err(QSQLError::ParseError {
                message: "Incomplete DELETE statement".to_string(),
                position: i,
            });
        };

        // Parse WHERE clause
        let mut where_clause = None;
        if i < tokens.len() && matches!(tokens[i], TokenType::Where) {
            i += 1;
            where_clause = Some(self.parse_expression(tokens, &mut i)?);
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

    /// Parse LEARN PATTERN statement
    fn parse_learn_pattern_statement(&self, tokens: &[TokenType]) -> QSQLResult<Statement> {
        let mut target_table = String::new();
        let pattern_expression = None;
        let mut learning_rate = None;
        let mut epochs = None;
        let mut algorithm = None;

        // Parse LEARN PATTERN statement
        let mut i = 0;

        // Skip LEARN PATTERN keywords
        if i < tokens.len() && matches!(tokens[i], TokenType::Learn) {
            i += 1;
        }
        if i < tokens.len() && matches!(tokens[i], TokenType::Pattern) {
            i += 1;
        }

        // Get target table
        if i < tokens.len() {
            if let TokenType::Identifier(table_name) = &tokens[i] {
                target_table = table_name.clone();
                i += 1;
            }
        }

        // Parse optional learning parameters
        while i < tokens.len() {
            match &tokens[i] {
                TokenType::LearningRate => {
                    i += 1;
                    if i < tokens.len() {
                        if let TokenType::FloatLiteral(rate) = &tokens[i] {
                            learning_rate = Some(*rate);
                            i += 1;
                        }
                    }
                }
                TokenType::Epochs => {
                    i += 1;
                    if i < tokens.len() {
                        if let TokenType::IntegerLiteral(e) = &tokens[i] {
                            epochs = Some(*e as u64);
                            i += 1;
                        }
                    }
                }
                TokenType::Algorithm => {
                    i += 1;
                    if i < tokens.len() {
                        if let TokenType::Identifier(alg) = &tokens[i] {
                            algorithm = Some(alg.clone());
                            i += 1;
                        }
                    }
                }
                _ => break, // Exit on unexpected token
            }
        }

        let learn_pattern = LearnPatternStatement {
            target_table,
            pattern_expression,
            learning_rate,
            epochs,
            algorithm,
        };

        Ok(Statement::LearnPattern(learn_pattern))
    }

    /// Parse ADAPT WEIGHTS statement
    fn parse_adapt_weights_statement(&self, tokens: &[TokenType]) -> QSQLResult<Statement> {
        let mut target_table = String::new();
        let mut weight_expression = None;
        let mut plasticity_threshold = None;
        let mut hebbian_strengthening = false;

        // Parse ADAPT WEIGHTS statement
        let mut i = 0;

        // Skip ADAPT WEIGHTS keywords
        if i < tokens.len() && matches!(tokens[i], TokenType::Adapt) {
            i += 1;
        }
        if i < tokens.len() && matches!(tokens[i], TokenType::Weights) {
            i += 1;
        }

        // Get target table
        if i < tokens.len() {
            if let TokenType::Identifier(table_name) = &tokens[i] {
                target_table = table_name.clone();
                i += 1;
            }
        }

        // Parse optional weight expression
        if i < tokens.len() {
            weight_expression = Some(self.parse_expression(tokens, &mut i)?);
        }

        // Parse optional plasticity threshold
        if i < tokens.len() && matches!(tokens[i], TokenType::PlasticityThreshold) {
            i += 1;
            if i < tokens.len() {
                if let TokenType::FloatLiteral(threshold) = &tokens[i] {
                    plasticity_threshold = Some(*threshold);
                    i += 1;
                }
            }
        }

        // Check for optional HEBBIAN_LEARNING keyword
        if i < tokens.len() && matches!(tokens[i], TokenType::HebbianLearning) {
            hebbian_strengthening = true;
        }

        let adapt_weights = AdaptWeightsStatement {
            target_table,
            weight_expression,
            plasticity_threshold,
            hebbian_strengthening,
        };

        Ok(Statement::AdaptWeights(adapt_weights))
    }

    /// Parse QUANTUM_JOIN statement
    fn parse_quantum_join_statement(&self, tokens: &[TokenType]) -> QSQLResult<Statement> {
        let mut left_table = String::new();
        let mut right_table = String::new();
        let mut on_condition = None;
        let mut using_columns = Vec::new();
        let mut quantum_state = None;

        // Parse QUANTUM_JOIN statement
        let mut i = 0;

        // Skip QUANTUM_JOIN keywords
        if i < tokens.len() && matches!(tokens[i], TokenType::QuantumJoin) {
            i += 1;
        }

        // Parse LEFT table
        if i < tokens.len() {
            if let TokenType::Identifier(table_name) = &tokens[i] {
                left_table = table_name.clone();
                i += 1;
            }
        }

        // Parse RIGHT table
        if i < tokens.len() {
            if let TokenType::Identifier(table_name) = &tokens[i] {
                right_table = table_name.clone();
                i += 1;
            }
        }

        // Parse ON condition (optional)
        if i < tokens.len() && matches!(tokens[i], TokenType::On) {
            i += 1;
            on_condition = Some(self.parse_expression(tokens, &mut i)?);
        }

        // Parse USING columns (optional)
        if i < tokens.len() && matches!(tokens[i], TokenType::Using) {
            i += 1;
            if i < tokens.len() && matches!(tokens[i], TokenType::LeftParen) {
                i += 1; // Skip '('
                loop {
                    if i >= tokens.len() {
                        return Err(QSQLError::ParseError {
                            message: "Unclosed USING column list".to_string(),
                            position: i,
                        });
                    }

                    if matches!(tokens[i], TokenType::RightParen) {
                        i += 1; // Skip ')'
                        break;
                    }

                    if let TokenType::Identifier(col_name) = &tokens[i] {
                        using_columns.push(col_name.clone());
                        i += 1;

                        if i < tokens.len() && matches!(tokens[i], TokenType::Comma) {
                            i += 1; // Skip ','
                        }
                    } else {
                        return Err(QSQLError::ParseError {
                            message: "Expected column name in USING clause".to_string(),
                            position: i,
                        });
                    }
                }
            }
        }

        // Parse optional quantum state
        if i < tokens.len() && matches!(tokens[i], TokenType::QuantumState) {
            i += 1;
            if i < tokens.len() {
                if let TokenType::Identifier(state_name) = &tokens[i] {
                    quantum_state = Some(state_name.clone());
                }
            }
        }

        let quantum_join = QuantumJoinStatement {
            left_table,
            right_table,
            on_condition,
            using_columns,
            quantum_state,
        };

        Ok(Statement::QuantumJoin(quantum_join))
    }

    /// Parse BEGIN or BEGIN TRANSACTION statement
    fn parse_begin_transaction(&self, tokens: &[TokenType]) -> QSQLResult<Statement> {
        let mut i = 0;

        // Skip BEGIN keyword
        if i < tokens.len() && matches!(tokens[i], TokenType::Begin) {
            i += 1;
        }

        // Skip optional TRANSACTION keyword
        if i < tokens.len() && matches!(tokens[i], TokenType::Transaction) {
            i += 1;
        }

        // Silence unused variable warning (i would be used if we parsed isolation level)
        let _ = i;

        // Parse optional isolation level (e.g., ISOLATION LEVEL SERIALIZABLE)
        let isolation_level = None; // TODO: Parse isolation level if needed

        Ok(Statement::BeginTransaction(
            crate::ast::BeginTransactionStatement { isolation_level },
        ))
    }

    /// Parse START TRANSACTION statement
    fn parse_start_transaction(&self, tokens: &[TokenType]) -> QSQLResult<Statement> {
        let mut i = 0;

        // Skip START keyword
        if i < tokens.len() && matches!(tokens[i], TokenType::Start) {
            i += 1;
        }

        // Skip TRANSACTION keyword
        if i < tokens.len() && matches!(tokens[i], TokenType::Transaction) {
            i += 1;
        }

        // Silence unused variable warning
        let _ = i;

        Ok(Statement::BeginTransaction(
            crate::ast::BeginTransactionStatement {
                isolation_level: None,
            },
        ))
    }

    /// Parse COMMIT statement
    fn parse_commit(&self, _tokens: &[TokenType]) -> QSQLResult<Statement> {
        Ok(Statement::Commit(crate::ast::CommitStatement {}))
    }

    /// Parse ROLLBACK statement (with optional TO SAVEPOINT)
    fn parse_rollback(&self, tokens: &[TokenType]) -> QSQLResult<Statement> {
        let mut i = 0;

        // Skip ROLLBACK keyword
        if i < tokens.len() && matches!(tokens[i], TokenType::Rollback) {
            i += 1;
        }

        // Check for TO SAVEPOINT
        if i < tokens.len() && matches!(tokens[i], TokenType::To) {
            i += 1; // skip TO

            // Skip optional SAVEPOINT keyword
            if i < tokens.len() && matches!(tokens[i], TokenType::Savepoint) {
                i += 1;
            }

            // Get savepoint name
            if let Some(TokenType::Identifier(name)) = tokens.get(i) {
                return Ok(Statement::RollbackToSavepoint(
                    crate::ast::RollbackToSavepointStatement { name: name.clone() },
                ));
            } else {
                return Err(QSQLError::ParseError {
                    message: "Expected savepoint name after ROLLBACK TO".to_string(),
                    position: i,
                });
            }
        }

        Ok(Statement::Rollback(crate::ast::RollbackStatement {}))
    }

    /// Parse SAVEPOINT statement
    fn parse_savepoint(&self, tokens: &[TokenType]) -> QSQLResult<Statement> {
        let mut i = 0;

        // Skip SAVEPOINT keyword
        if i < tokens.len() && matches!(tokens[i], TokenType::Savepoint) {
            i += 1;
        }

        // Get savepoint name
        if let Some(TokenType::Identifier(name)) = tokens.get(i) {
            Ok(Statement::Savepoint(crate::ast::SavepointStatement {
                name: name.clone(),
            }))
        } else {
            Err(QSQLError::ParseError {
                message: "Expected savepoint name after SAVEPOINT".to_string(),
                position: i,
            })
        }
    }

    /// Parse RELEASE SAVEPOINT statement
    fn parse_release_savepoint(&self, tokens: &[TokenType]) -> QSQLResult<Statement> {
        let mut i = 0;

        // Skip RELEASE keyword
        if i < tokens.len() && matches!(tokens[i], TokenType::Release) {
            i += 1;
        }

        // Skip optional SAVEPOINT keyword
        if i < tokens.len() && matches!(tokens[i], TokenType::Savepoint) {
            i += 1;
        }

        // Get savepoint name
        if let Some(TokenType::Identifier(name)) = tokens.get(i) {
            Ok(Statement::ReleaseSavepoint(
                crate::ast::ReleaseSavepointStatement { name: name.clone() },
            ))
        } else {
            Err(QSQLError::ParseError {
                message: "Expected savepoint name after RELEASE".to_string(),
                position: i,
            })
        }
    }

    /// Parse CREATE statement (TABLE or INDEX)
    fn parse_create_statement(&self, tokens: &[TokenType]) -> QSQLResult<Statement> {
        if tokens.len() < 2 {
            return Err(QSQLError::ParseError {
                message: "Incomplete CREATE statement".to_string(),
                position: 0,
            });
        }

        match &tokens[1] {
            TokenType::Table => self.parse_create_table_statement(tokens),
            TokenType::Index | TokenType::Unique => self.parse_create_index_statement(tokens),
            _ => Err(QSQLError::ParseError {
                message: "Expected TABLE or INDEX after CREATE".to_string(),
                position: 1,
            }),
        }
    }

    /// Parse CREATE TABLE statement
    fn parse_create_table_statement(&self, tokens: &[TokenType]) -> QSQLResult<Statement> {
        let mut i = 0;

        // Skip CREATE keyword
        if i < tokens.len() && matches!(tokens[i], TokenType::Create) {
            i += 1;
        }

        // Skip TABLE keyword
        if i < tokens.len() && matches!(tokens[i], TokenType::Table) {
            i += 1;
        }

        // Check for IF NOT EXISTS
        let mut if_not_exists = false;
        if i + 2 < tokens.len()
            && matches!(tokens[i], TokenType::If)
            && matches!(tokens[i + 1], TokenType::Not)
            && matches!(tokens[i + 2], TokenType::Exists)
        {
            if_not_exists = true;
            i += 3;
        }

        // Parse table name
        let table_name = if let TokenType::Identifier(name) = &tokens[i] {
            i += 1;
            name.clone()
        } else {
            return Err(QSQLError::ParseError {
                message: "Expected table name".to_string(),
                position: i,
            });
        };

        // Expect '('
        if i >= tokens.len() || !matches!(tokens[i], TokenType::LeftParen) {
            return Err(QSQLError::ParseError {
                message: "Expected '(' after table name".to_string(),
                position: i,
            });
        }
        i += 1;

        // Parse column definitions and constraints
        let mut columns = Vec::new();
        let mut constraints = Vec::new();

        while i < tokens.len() && !matches!(tokens[i], TokenType::RightParen) {
            // Check if this is a table constraint
            if matches!(tokens[i], TokenType::Primary | TokenType::Unique) {
                // Parse table constraint
                let constraint = self.parse_table_constraint(tokens, &mut i)?;
                constraints.push(constraint);
            } else if let TokenType::Identifier(_) = &tokens[i] {
                // Parse column definition
                let column = self.parse_column_definition(tokens, &mut i)?;
                columns.push(column);
            } else {
                return Err(QSQLError::ParseError {
                    message: format!("Unexpected token in CREATE TABLE: {:?}", tokens[i]),
                    position: i,
                });
            }

            // Handle comma separator
            if i < tokens.len() && matches!(tokens[i], TokenType::Comma) {
                i += 1;
            } else if i < tokens.len() && !matches!(tokens[i], TokenType::RightParen) {
                return Err(QSQLError::ParseError {
                    message: "Expected ',' or ')' in column list".to_string(),
                    position: i,
                });
            }
        }

        // Expect ')'
        if i >= tokens.len() || !matches!(tokens[i], TokenType::RightParen) {
            return Err(QSQLError::ParseError {
                message: "Expected ')' after column definitions".to_string(),
                position: i,
            });
        }

        Ok(Statement::CreateTable(CreateTableStatement {
            table_name,
            if_not_exists,
            columns,
            constraints,
            synaptic_indexing: false,
            plasticity_config: None,
        }))
    }

    /// Parse column definition
    fn parse_column_definition(
        &self,
        tokens: &[TokenType],
        i: &mut usize,
    ) -> QSQLResult<ColumnDefinition> {
        // Parse column name
        let name = if let TokenType::Identifier(col_name) = &tokens[*i] {
            *i += 1;
            col_name.clone()
        } else {
            return Err(QSQLError::ParseError {
                message: "Expected column name".to_string(),
                position: *i,
            });
        };

        // Parse data type
        let data_type = self.parse_data_type(tokens, i)?;

        // Parse column constraints
        let mut constraints = Vec::new();
        while *i < tokens.len() {
            let constraint = match &tokens[*i] {
                TokenType::Primary => {
                    *i += 1;
                    if *i < tokens.len() && matches!(tokens[*i], TokenType::Key) {
                        *i += 1;
                    }
                    ColumnConstraint::PrimaryKey
                }
                TokenType::Not => {
                    *i += 1;
                    if *i < tokens.len() && matches!(tokens[*i], TokenType::Null) {
                        *i += 1;
                        ColumnConstraint::NotNull
                    } else {
                        return Err(QSQLError::ParseError {
                            message: "Expected NULL after NOT".to_string(),
                            position: *i,
                        });
                    }
                }
                TokenType::Unique => {
                    *i += 1;
                    ColumnConstraint::Unique
                }
                TokenType::Default => {
                    *i += 1;
                    let default_expr = self.parse_expression(tokens, i)?;
                    ColumnConstraint::Default(default_expr)
                }
                TokenType::AutoIncrement => {
                    *i += 1;
                    ColumnConstraint::AutoIncrement
                }
                TokenType::References => {
                    *i += 1;
                    let table = if let TokenType::Identifier(t) = &tokens[*i] {
                        *i += 1;
                        t.clone()
                    } else {
                        return Err(QSQLError::ParseError {
                            message: "Expected table name after REFERENCES".to_string(),
                            position: *i,
                        });
                    };

                    // Expect '('
                    if *i >= tokens.len() || !matches!(tokens[*i], TokenType::LeftParen) {
                        return Err(QSQLError::ParseError {
                            message: "Expected '(' after referenced table".to_string(),
                            position: *i,
                        });
                    }
                    *i += 1;

                    let column = if let TokenType::Identifier(c) = &tokens[*i] {
                        *i += 1;
                        c.clone()
                    } else {
                        return Err(QSQLError::ParseError {
                            message: "Expected column name in REFERENCES".to_string(),
                            position: *i,
                        });
                    };

                    // Expect ')'
                    if *i >= tokens.len() || !matches!(tokens[*i], TokenType::RightParen) {
                        return Err(QSQLError::ParseError {
                            message: "Expected ')' after referenced column".to_string(),
                            position: *i,
                        });
                    }
                    *i += 1;

                    ColumnConstraint::ForeignKey { table, column }
                }
                _ => break,
            };
            constraints.push(constraint);
        }

        Ok(ColumnDefinition {
            name,
            data_type,
            constraints,
            synaptic_properties: None,
        })
    }

    /// Parse data type
    fn parse_data_type(&self, tokens: &[TokenType], i: &mut usize) -> QSQLResult<DataType> {
        if *i >= tokens.len() {
            return Err(QSQLError::ParseError {
                message: "Expected data type".to_string(),
                position: *i,
            });
        }

        let data_type = match &tokens[*i] {
            TokenType::Identifier(type_name) => {
                *i += 1;
                match type_name.to_uppercase().as_str() {
                    "INTEGER" | "INT" => DataType::Integer,
                    "BIGINT" => DataType::BigInt,
                    "SMALLINT" => DataType::SmallInt,
                    "REAL" | "FLOAT" => DataType::Real,
                    "DOUBLE" => DataType::Double,
                    "TEXT" => DataType::Text,
                    "BOOLEAN" | "BOOL" => DataType::Boolean,
                    "DATE" => DataType::Date,
                    "TIME" => DataType::Time,
                    "TIMESTAMP" => DataType::Timestamp,
                    "BLOB" => DataType::Blob,
                    "VARCHAR" => {
                        if *i < tokens.len() && matches!(tokens[*i], TokenType::LeftParen) {
                            *i += 1;
                            if let TokenType::IntegerLiteral(size) = tokens[*i] {
                                *i += 1;
                                if *i < tokens.len() && matches!(tokens[*i], TokenType::RightParen)
                                {
                                    *i += 1;
                                    DataType::Varchar(size as u32)
                                } else {
                                    return Err(QSQLError::ParseError {
                                        message: "Expected ')' after VARCHAR size".to_string(),
                                        position: *i,
                                    });
                                }
                            } else {
                                return Err(QSQLError::ParseError {
                                    message: "Expected size for VARCHAR".to_string(),
                                    position: *i,
                                });
                            }
                        } else {
                            DataType::VarChar(None)
                        }
                    }
                    "DECIMAL" => {
                        if *i < tokens.len() && matches!(tokens[*i], TokenType::LeftParen) {
                            *i += 1;
                            let precision = if let TokenType::IntegerLiteral(p) = tokens[*i] {
                                *i += 1;
                                p as u8
                            } else {
                                return Err(QSQLError::ParseError {
                                    message: "Expected precision for DECIMAL".to_string(),
                                    position: *i,
                                });
                            };

                            let scale =
                                if *i < tokens.len() && matches!(tokens[*i], TokenType::Comma) {
                                    *i += 1;
                                    if let TokenType::IntegerLiteral(s) = tokens[*i] {
                                        *i += 1;
                                        s as u8
                                    } else {
                                        return Err(QSQLError::ParseError {
                                            message: "Expected scale for DECIMAL".to_string(),
                                            position: *i,
                                        });
                                    }
                                } else {
                                    0
                                };

                            if *i < tokens.len() && matches!(tokens[*i], TokenType::RightParen) {
                                *i += 1;
                                DataType::Decimal(precision, scale)
                            } else {
                                return Err(QSQLError::ParseError {
                                    message: "Expected ')' after DECIMAL parameters".to_string(),
                                    position: *i,
                                });
                            }
                        } else {
                            DataType::Decimal(10, 2)
                        }
                    }
                    _ => {
                        return Err(QSQLError::ParseError {
                            message: format!("Unknown data type: {}", type_name),
                            position: *i - 1,
                        })
                    }
                }
            }
            TokenType::Serial => {
                *i += 1;
                DataType::Serial
            }
            TokenType::BigSerial => {
                *i += 1;
                DataType::BigSerial
            }
            TokenType::SmallSerial => {
                *i += 1;
                DataType::SmallSerial
            }
            _ => {
                return Err(QSQLError::ParseError {
                    message: format!("Expected data type, found {:?}", tokens[*i]),
                    position: *i,
                })
            }
        };

        Ok(data_type)
    }

    /// Parse table constraint (PRIMARY KEY, UNIQUE, etc.)
    fn parse_table_constraint(
        &self,
        tokens: &[TokenType],
        i: &mut usize,
    ) -> QSQLResult<TableConstraint> {
        match &tokens[*i] {
            TokenType::Primary => {
                *i += 1;
                if *i < tokens.len() && matches!(tokens[*i], TokenType::Key) {
                    *i += 1;
                }

                // Expect '('
                if *i >= tokens.len() || !matches!(tokens[*i], TokenType::LeftParen) {
                    return Err(QSQLError::ParseError {
                        message: "Expected '(' after PRIMARY KEY".to_string(),
                        position: *i,
                    });
                }
                *i += 1;

                // Parse column list
                let mut columns = Vec::new();
                while *i < tokens.len() && !matches!(tokens[*i], TokenType::RightParen) {
                    if let TokenType::Identifier(col) = &tokens[*i] {
                        columns.push(col.clone());
                        *i += 1;

                        if *i < tokens.len() && matches!(tokens[*i], TokenType::Comma) {
                            *i += 1;
                        }
                    } else {
                        return Err(QSQLError::ParseError {
                            message: "Expected column name in PRIMARY KEY".to_string(),
                            position: *i,
                        });
                    }
                }

                // Expect ')'
                if *i >= tokens.len() || !matches!(tokens[*i], TokenType::RightParen) {
                    return Err(QSQLError::ParseError {
                        message: "Expected ')' after PRIMARY KEY columns".to_string(),
                        position: *i,
                    });
                }
                *i += 1;

                Ok(TableConstraint::PrimaryKey(columns))
            }
            TokenType::Unique => {
                *i += 1;

                // Expect '('
                if *i >= tokens.len() || !matches!(tokens[*i], TokenType::LeftParen) {
                    return Err(QSQLError::ParseError {
                        message: "Expected '(' after UNIQUE".to_string(),
                        position: *i,
                    });
                }
                *i += 1;

                // Parse column list
                let mut columns = Vec::new();
                while *i < tokens.len() && !matches!(tokens[*i], TokenType::RightParen) {
                    if let TokenType::Identifier(col) = &tokens[*i] {
                        columns.push(col.clone());
                        *i += 1;

                        if *i < tokens.len() && matches!(tokens[*i], TokenType::Comma) {
                            *i += 1;
                        }
                    } else {
                        return Err(QSQLError::ParseError {
                            message: "Expected column name in UNIQUE".to_string(),
                            position: *i,
                        });
                    }
                }

                // Expect ')'
                if *i >= tokens.len() || !matches!(tokens[*i], TokenType::RightParen) {
                    return Err(QSQLError::ParseError {
                        message: "Expected ')' after UNIQUE columns".to_string(),
                        position: *i,
                    });
                }
                *i += 1;

                Ok(TableConstraint::Unique(columns))
            }
            _ => Err(QSQLError::ParseError {
                message: format!("Unexpected constraint token: {:?}", tokens[*i]),
                position: *i,
            }),
        }
    }

    /// Parse DROP statement (TABLE or INDEX)
    fn parse_drop_statement(&self, tokens: &[TokenType]) -> QSQLResult<Statement> {
        if tokens.len() < 2 {
            return Err(QSQLError::ParseError {
                message: "Incomplete DROP statement".to_string(),
                position: 0,
            });
        }

        match &tokens[1] {
            TokenType::Table => self.parse_drop_table_statement(tokens),
            TokenType::Index => self.parse_drop_index_statement(tokens),
            _ => Err(QSQLError::ParseError {
                message: "Expected TABLE or INDEX after DROP".to_string(),
                position: 1,
            }),
        }
    }

    /// Parse DROP TABLE statement
    fn parse_drop_table_statement(&self, tokens: &[TokenType]) -> QSQLResult<Statement> {
        let mut i = 0;

        // Skip DROP keyword
        if i < tokens.len() && matches!(tokens[i], TokenType::Drop) {
            i += 1;
        }

        // Skip TABLE keyword
        if i < tokens.len() && matches!(tokens[i], TokenType::Table) {
            i += 1;
        }

        // Check for IF EXISTS
        let mut if_exists = false;
        if i + 1 < tokens.len()
            && matches!(tokens[i], TokenType::If)
            && matches!(tokens[i + 1], TokenType::Exists)
        {
            if_exists = true;
            i += 2;
        }

        // Parse table name
        let table_name = if let TokenType::Identifier(name) = &tokens[i] {
            name.clone()
        } else {
            return Err(QSQLError::ParseError {
                message: "Expected table name".to_string(),
                position: i,
            });
        };

        Ok(Statement::DropTable(DropTableStatement {
            table_name,
            if_exists,
            preserve_synaptic_patterns: false,
        }))
    }

    /// Parse ALTER TABLE statement
    fn parse_alter_table_statement(&self, tokens: &[TokenType]) -> QSQLResult<Statement> {
        let mut i = 0;

        // Skip ALTER keyword
        if i < tokens.len() && matches!(tokens[i], TokenType::Alter) {
            i += 1;
        }

        // Skip TABLE keyword
        if i < tokens.len() && matches!(tokens[i], TokenType::Table) {
            i += 1;
        }

        // Parse table name
        let table_name = if let TokenType::Identifier(name) = &tokens[i] {
            i += 1;
            name.clone()
        } else {
            return Err(QSQLError::ParseError {
                message: "Expected table name".to_string(),
                position: i,
            });
        };

        // Parse operation (ADD, DROP, MODIFY)
        let operation = match &tokens[i] {
            TokenType::Add => {
                i += 1;
                // Skip optional COLUMN keyword
                if i < tokens.len() && matches!(tokens[i], TokenType::Column) {
                    i += 1;
                }

                let column = self.parse_column_definition(tokens, &mut i)?;
                AlterTableOperation::AddColumn { column }
            }
            TokenType::Drop => {
                i += 1;
                // Skip optional COLUMN keyword
                if i < tokens.len() && matches!(tokens[i], TokenType::Column) {
                    i += 1;
                }

                let column_name = if let TokenType::Identifier(name) = &tokens[i] {
                    name.clone()
                } else {
                    return Err(QSQLError::ParseError {
                        message: "Expected column name after DROP".to_string(),
                        position: i,
                    });
                };

                AlterTableOperation::DropColumn { column_name }
            }
            TokenType::Modify => {
                i += 1;
                // Skip optional COLUMN keyword
                if i < tokens.len() && matches!(tokens[i], TokenType::Column) {
                    i += 1;
                }

                let column_name = if let TokenType::Identifier(name) = &tokens[i] {
                    i += 1;
                    name.clone()
                } else {
                    return Err(QSQLError::ParseError {
                        message: "Expected column name after MODIFY".to_string(),
                        position: i,
                    });
                };

                let new_data_type = self.parse_data_type(tokens, &mut i)?;

                AlterTableOperation::ModifyColumn {
                    column_name,
                    new_data_type,
                }
            }
            _ => {
                return Err(QSQLError::ParseError {
                    message: "Expected ADD, DROP, or MODIFY after ALTER TABLE".to_string(),
                    position: i,
                })
            }
        };

        Ok(Statement::AlterTable(AlterTableStatement {
            table_name,
            operation,
        }))
    }

    /// Parse CREATE INDEX statement
    fn parse_create_index_statement(&self, tokens: &[TokenType]) -> QSQLResult<Statement> {
        let mut i = 0;

        // Skip CREATE keyword
        if i < tokens.len() && matches!(tokens[i], TokenType::Create) {
            i += 1;
        }

        // Check for UNIQUE
        let unique = if i < tokens.len() && matches!(tokens[i], TokenType::Unique) {
            i += 1;
            true
        } else {
            false
        };

        // Skip INDEX keyword
        if i < tokens.len() && matches!(tokens[i], TokenType::Index) {
            i += 1;
        }

        // Check for IF NOT EXISTS
        let mut if_not_exists = false;
        if i + 2 < tokens.len()
            && matches!(tokens[i], TokenType::If)
            && matches!(tokens[i + 1], TokenType::Not)
            && matches!(tokens[i + 2], TokenType::Exists)
        {
            if_not_exists = true;
            i += 3;
        }

        // Parse index name
        let index_name = if let TokenType::Identifier(name) = &tokens[i] {
            i += 1;
            name.clone()
        } else {
            return Err(QSQLError::ParseError {
                message: "Expected index name".to_string(),
                position: i,
            });
        };

        // Expect ON
        let on_keyword_found = if i < tokens.len() {
            matches!(tokens[i], TokenType::On) || {
                if let TokenType::Identifier(s) = &tokens[i] {
                    s.to_uppercase() == "ON"
                } else {
                    false
                }
            }
        } else {
            false
        };

        if !on_keyword_found {
            return Err(QSQLError::ParseError {
                message: "Expected ON after index name".to_string(),
                position: i,
            });
        }
        i += 1;

        // Parse table name
        let table_name = if let TokenType::Identifier(name) = &tokens[i] {
            i += 1;
            name.clone()
        } else {
            return Err(QSQLError::ParseError {
                message: "Expected table name after ON".to_string(),
                position: i,
            });
        };

        // Expect '('
        if i >= tokens.len() || !matches!(tokens[i], TokenType::LeftParen) {
            return Err(QSQLError::ParseError {
                message: "Expected '(' after table name".to_string(),
                position: i,
            });
        }
        i += 1;

        // Parse column list
        let mut columns = Vec::new();
        while i < tokens.len() && !matches!(tokens[i], TokenType::RightParen) {
            if let TokenType::Identifier(col) = &tokens[i] {
                columns.push(col.clone());
                i += 1;

                if i < tokens.len() && matches!(tokens[i], TokenType::Comma) {
                    i += 1;
                }
            } else {
                return Err(QSQLError::ParseError {
                    message: "Expected column name in index".to_string(),
                    position: i,
                });
            }
        }

        // Expect ')'
        if i >= tokens.len() || !matches!(tokens[i], TokenType::RightParen) {
            return Err(QSQLError::ParseError {
                message: "Expected ')' after index columns".to_string(),
                position: i,
            });
        }

        Ok(Statement::CreateIndex(CreateIndexStatement {
            index_name,
            table_name,
            columns,
            unique,
            if_not_exists,
        }))
    }

    /// Parse DROP INDEX statement
    fn parse_drop_index_statement(&self, tokens: &[TokenType]) -> QSQLResult<Statement> {
        let mut i = 0;

        // Skip DROP keyword
        if i < tokens.len() && matches!(tokens[i], TokenType::Drop) {
            i += 1;
        }

        // Skip INDEX keyword
        if i < tokens.len() && matches!(tokens[i], TokenType::Index) {
            i += 1;
        }

        // Check for IF EXISTS
        let mut if_exists = false;
        if i + 1 < tokens.len()
            && matches!(tokens[i], TokenType::If)
            && matches!(tokens[i + 1], TokenType::Exists)
        {
            if_exists = true;
            i += 2;
        }

        // Parse index name
        let index_name = if let TokenType::Identifier(name) = &tokens[i] {
            name.clone()
        } else {
            return Err(QSQLError::ParseError {
                message: "Expected index name".to_string(),
                position: i,
            });
        };

        Ok(Statement::DropIndex(DropIndexStatement {
            index_name,
            if_exists,
        }))
    }

    /// Parse TRUNCATE TABLE statement
    fn parse_truncate_table_statement(&self, tokens: &[TokenType]) -> QSQLResult<Statement> {
        let mut i = 0;

        // Skip TRUNCATE keyword
        if i < tokens.len() && matches!(tokens[i], TokenType::Truncate) {
            i += 1;
        }

        // Skip optional TABLE keyword
        if i < tokens.len() && matches!(tokens[i], TokenType::Table) {
            i += 1;
        }

        // Parse table name
        let table_name = if let TokenType::Identifier(name) = &tokens[i] {
            name.clone()
        } else {
            return Err(QSQLError::ParseError {
                message: "Expected table name".to_string(),
                position: i,
            });
        };

        Ok(Statement::TruncateTable(TruncateTableStatement {
            table_name,
        }))
    }

    /// Parse an optional table alias (AS alias or just identifier after table name)
    fn parse_table_alias(&self, tokens: &[TokenType], i: &mut usize) -> Option<String> {
        if *i >= tokens.len() {
            return None;
        }

        // Check for AS keyword
        if let TokenType::As = &tokens[*i] {
            *i += 1;
            if *i < tokens.len() {
                if let TokenType::Identifier(alias) = &tokens[*i] {
                    *i += 1;
                    return Some(alias.clone());
                }
            }
            return None;
        }

        // Check for direct identifier as alias (not a keyword)
        if let TokenType::Identifier(alias) = &tokens[*i] {
            // Avoid treating SQL keywords as aliases
            let upper = alias.to_uppercase();
            if !matches!(
                upper.as_str(),
                "WHERE"
                    | "JOIN"
                    | "INNER"
                    | "LEFT"
                    | "RIGHT"
                    | "FULL"
                    | "CROSS"
                    | "ON"
                    | "ORDER"
                    | "GROUP"
                    | "HAVING"
                    | "LIMIT"
                    | "OFFSET"
                    | "UNION"
                    | "INTERSECT"
                    | "EXCEPT"
            ) {
                *i += 1;
                return Some(alias.clone());
            }
        }

        None
    }

    /// Parse WITH clause for Common Table Expressions (CTEs)
    fn parse_with_clause(&self, tokens: &[TokenType], i: &mut usize) -> QSQLResult<WithClause> {
        // Consume WITH keyword
        if *i >= tokens.len() || !matches!(tokens[*i], TokenType::With) {
            return Err(QSQLError::ParseError {
                message: "Expected WITH keyword".to_string(),
                position: *i,
            });
        }
        *i += 1;

        // Check for optional RECURSIVE keyword
        let recursive = if *i < tokens.len() && matches!(tokens[*i], TokenType::Recursive) {
            *i += 1;
            true
        } else {
            false
        };

        let mut ctes = Vec::new();

        // Parse one or more CTEs
        loop {
            // Parse CTE name
            let name = if let Some(TokenType::Identifier(cte_name)) = tokens.get(*i) {
                let name = cte_name.clone();
                *i += 1;
                name
            } else {
                return Err(QSQLError::ParseError {
                    message: "Expected CTE name after WITH".to_string(),
                    position: *i,
                });
            };

            // Parse optional column list: (col1, col2, ...)
            let columns = if *i < tokens.len() && matches!(tokens[*i], TokenType::LeftParen) {
                *i += 1; // consume '('
                let mut cols = Vec::new();

                loop {
                    if *i >= tokens.len() {
                        return Err(QSQLError::ParseError {
                            message: "Unclosed column list in CTE".to_string(),
                            position: *i,
                        });
                    }

                    // Check for closing paren
                    if matches!(tokens[*i], TokenType::RightParen) {
                        *i += 1; // consume ')'
                        break;
                    }

                    // Parse column name
                    if let TokenType::Identifier(col_name) = &tokens[*i] {
                        cols.push(col_name.clone());
                        *i += 1;

                        // Check for comma or closing paren
                        if *i < tokens.len() && matches!(tokens[*i], TokenType::Comma) {
                            *i += 1; // consume ','
                        } else if *i < tokens.len() && matches!(tokens[*i], TokenType::RightParen) {
                            *i += 1; // consume ')'
                            break;
                        }
                    } else {
                        return Err(QSQLError::ParseError {
                            message: "Expected column name in CTE column list".to_string(),
                            position: *i,
                        });
                    }
                }

                Some(cols)
            } else {
                None
            };

            // Expect AS keyword
            if *i >= tokens.len() || !matches!(tokens[*i], TokenType::As) {
                return Err(QSQLError::ParseError {
                    message: "Expected AS keyword after CTE name".to_string(),
                    position: *i,
                });
            }
            *i += 1;

            // Expect opening paren for CTE query
            if *i >= tokens.len() || !matches!(tokens[*i], TokenType::LeftParen) {
                return Err(QSQLError::ParseError {
                    message: "Expected '(' after AS in CTE".to_string(),
                    position: *i,
                });
            }
            *i += 1;

            // Parse the SELECT statement inside the CTE
            let query = self.parse_select_statement_at(tokens, i)?;

            // Expect closing paren
            if *i >= tokens.len() || !matches!(tokens[*i], TokenType::RightParen) {
                return Err(QSQLError::ParseError {
                    message: "Expected ')' after CTE query".to_string(),
                    position: *i,
                });
            }
            *i += 1;

            // Add the CTE to the list
            ctes.push(CommonTableExpression {
                name,
                query: Box::new(query),
                columns,
            });

            // Check for comma (more CTEs) or end of WITH clause
            if *i < tokens.len() && matches!(tokens[*i], TokenType::Comma) {
                *i += 1; // consume ','
                continue; // Parse next CTE
            } else {
                break; // End of CTE list
            }
        }

        Ok(WithClause { recursive, ctes })
    }

    /// Parse JOIN clauses after the first table in FROM
    fn parse_join_clauses(
        &self,
        tokens: &[TokenType],
        i: &mut usize,
    ) -> QSQLResult<Vec<JoinClause>> {
        let mut joins = Vec::new();

        loop {
            if *i >= tokens.len() {
                break;
            }

            // Determine join type
            let join_type = match &tokens[*i] {
                TokenType::Inner => {
                    *i += 1;
                    // Expect JOIN after INNER
                    if *i < tokens.len() && matches!(tokens[*i], TokenType::Join) {
                        *i += 1;
                    }
                    JoinType::Inner
                }
                TokenType::Left => {
                    *i += 1;
                    // Optional OUTER keyword
                    if *i < tokens.len() {
                        if let TokenType::Identifier(s) = &tokens[*i] {
                            if s.to_uppercase() == "OUTER" {
                                *i += 1;
                            }
                        }
                    }
                    // Expect JOIN
                    if *i < tokens.len() && matches!(tokens[*i], TokenType::Join) {
                        *i += 1;
                    }
                    JoinType::Left
                }
                TokenType::Right => {
                    *i += 1;
                    // Optional OUTER keyword
                    if *i < tokens.len() {
                        if let TokenType::Identifier(s) = &tokens[*i] {
                            if s.to_uppercase() == "OUTER" {
                                *i += 1;
                            }
                        }
                    }
                    // Expect JOIN
                    if *i < tokens.len() && matches!(tokens[*i], TokenType::Join) {
                        *i += 1;
                    }
                    JoinType::Right
                }
                TokenType::Full => {
                    *i += 1;
                    // Optional OUTER keyword
                    if *i < tokens.len() {
                        if let TokenType::Identifier(s) = &tokens[*i] {
                            if s.to_uppercase() == "OUTER" {
                                *i += 1;
                            }
                        }
                    }
                    // Expect JOIN
                    if *i < tokens.len() && matches!(tokens[*i], TokenType::Join) {
                        *i += 1;
                    }
                    JoinType::Full
                }
                TokenType::Cross => {
                    *i += 1;
                    // Expect JOIN
                    if *i < tokens.len() && matches!(tokens[*i], TokenType::Join) {
                        *i += 1;
                    }
                    JoinType::Cross
                }
                TokenType::Join => {
                    // Plain JOIN defaults to INNER JOIN
                    *i += 1;
                    JoinType::Inner
                }
                _ => break, // No more joins
            };

            // Parse the table name or derived table for the join
            if *i >= tokens.len() {
                return Err(QSQLError::ParseError {
                    message: "Expected table name or subquery after JOIN".to_string(),
                    position: *i,
                });
            }

            // Check if this is a derived table (subquery) or a regular table
            let relation = if matches!(tokens[*i], TokenType::LeftParen) {
                // This is a derived table (subquery)
                self.parse_derived_table(tokens, i)?
            } else if let TokenType::Identifier(name) = &tokens[*i] {
                let table_name = name.clone();
                *i += 1;

                // Parse optional alias
                let alias = self.parse_table_alias(tokens, i);

                TableReference {
                    name: table_name,
                    alias,
                    synaptic_weight: None,
                    quantum_state: None,
                    subquery: None,
                }
            } else {
                return Err(QSQLError::ParseError {
                    message: "Expected table name or subquery after JOIN".to_string(),
                    position: *i,
                });
            };

            // Parse ON condition (not required for CROSS JOIN)
            let condition = if *i < tokens.len() && matches!(tokens[*i], TokenType::On) {
                *i += 1;
                Some(self.parse_expression(tokens, i)?)
            } else if matches!(join_type, JoinType::Cross) {
                None // CROSS JOIN doesn't require ON
            } else {
                // For other joins, ON is expected
                return Err(QSQLError::ParseError {
                    message: format!("Expected ON clause for {:?} JOIN", join_type),
                    position: *i,
                });
            };

            joins.push(JoinClause {
                join_type,
                relation,
                condition,
                quantum_entanglement: false,
                superposition_join: false,
            });
        }

        Ok(joins)
    }

    /// Parse a derived table (subquery in FROM clause)
    /// Syntax: (SELECT ...) AS alias
    fn parse_derived_table(
        &self,
        tokens: &[TokenType],
        i: &mut usize,
    ) -> QSQLResult<TableReference> {
        // Expect opening parenthesis
        if *i >= tokens.len() || !matches!(tokens[*i], TokenType::LeftParen) {
            return Err(QSQLError::ParseError {
                message: "Expected '(' for derived table".to_string(),
                position: *i,
            });
        }
        *i += 1; // consume '('

        // Parse the subquery (must start with SELECT)
        if *i >= tokens.len() || !matches!(tokens[*i], TokenType::Select) {
            return Err(QSQLError::ParseError {
                message: "Expected SELECT after '(' in derived table".to_string(),
                position: *i,
            });
        }

        let subquery = self.parse_select_statement_at(tokens, i)?;

        // Expect closing parenthesis
        if *i >= tokens.len() || !matches!(tokens[*i], TokenType::RightParen) {
            return Err(QSQLError::ParseError {
                message: "Expected ')' after subquery in derived table".to_string(),
                position: *i,
            });
        }
        *i += 1; // consume ')'

        // Parse alias (required for derived tables)
        // First check for optional AS keyword
        if *i < tokens.len() && matches!(tokens[*i], TokenType::As) {
            *i += 1; // consume 'AS'
        }

        // Parse the alias name
        let alias = if *i < tokens.len() {
            if let TokenType::Identifier(alias_name) = &tokens[*i] {
                let alias = alias_name.clone();
                *i += 1;
                Some(alias)
            } else {
                // Alias is technically required for derived tables in standard SQL
                // but we'll allow it without for flexibility
                None
            }
        } else {
            None
        };

        // For derived tables, alias is required in standard SQL
        if alias.is_none() {
            return Err(QSQLError::ParseError {
                message: "Derived tables require an alias".to_string(),
                position: *i,
            });
        }

        Ok(TableReference {
            name: String::new(), // Empty name for derived tables
            alias,
            synaptic_weight: None,
            quantum_state: None,
            subquery: Some(Box::new(subquery)),
        })
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
        keywords.insert("INNER".to_string(), TokenType::Inner);
        keywords.insert("LEFT".to_string(), TokenType::Left);
        keywords.insert("RIGHT".to_string(), TokenType::Right);
        keywords.insert("FULL".to_string(), TokenType::Full);
        keywords.insert("CROSS".to_string(), TokenType::Cross);
        keywords.insert(
            "OUTER".to_string(),
            TokenType::Identifier("OUTER".to_string()),
        );
        keywords.insert("ON".to_string(), TokenType::On);
        keywords.insert("AS".to_string(), TokenType::As);
        keywords.insert("AND".to_string(), TokenType::And);
        keywords.insert("OR".to_string(), TokenType::Or);
        keywords.insert("NOT".to_string(), TokenType::Not);
        keywords.insert("WITH".to_string(), TokenType::With);
        keywords.insert("RECURSIVE".to_string(), TokenType::Recursive);
        keywords.insert("DISTINCT".to_string(), TokenType::Distinct);
        keywords.insert("EXTRACT".to_string(), TokenType::Extract);
        keywords.insert("IN".to_string(), TokenType::In);
        keywords.insert("LIKE".to_string(), TokenType::Like);
        keywords.insert("BETWEEN".to_string(), TokenType::Between);
        keywords.insert("IS".to_string(), TokenType::Is);
        keywords.insert("NULL".to_string(), TokenType::Null);

        // CASE expression keywords
        keywords.insert("CASE".to_string(), TokenType::Case);
        keywords.insert("WHEN".to_string(), TokenType::When);
        keywords.insert("THEN".to_string(), TokenType::Then);
        keywords.insert("ELSE".to_string(), TokenType::Else);
        keywords.insert("END".to_string(), TokenType::End);

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

        // DDL keywords
        keywords.insert("CREATE".to_string(), TokenType::Create);
        keywords.insert("DROP".to_string(), TokenType::Drop);
        keywords.insert("ALTER".to_string(), TokenType::Alter);
        keywords.insert("TRUNCATE".to_string(), TokenType::Truncate);
        keywords.insert("TABLE".to_string(), TokenType::Table);
        keywords.insert("INDEX".to_string(), TokenType::Index);
        keywords.insert("ADD".to_string(), TokenType::Add);
        keywords.insert("COLUMN".to_string(), TokenType::Column);
        keywords.insert("MODIFY".to_string(), TokenType::Modify);
        keywords.insert("IF".to_string(), TokenType::If);
        keywords.insert("EXISTS".to_string(), TokenType::Exists);

        // Auto-increment and identity keywords
        keywords.insert("SERIAL".to_string(), TokenType::Serial);
        keywords.insert("BIGSERIAL".to_string(), TokenType::BigSerial);
        keywords.insert("SMALLSERIAL".to_string(), TokenType::SmallSerial);
        keywords.insert("AUTO_INCREMENT".to_string(), TokenType::AutoIncrement);
        keywords.insert("AUTOINCREMENT".to_string(), TokenType::AutoIncrement); // SQLite style
        keywords.insert("PRIMARY".to_string(), TokenType::Primary);
        keywords.insert("KEY".to_string(), TokenType::Key);
        keywords.insert("UNIQUE".to_string(), TokenType::Unique);
        keywords.insert("REFERENCES".to_string(), TokenType::References);
        keywords.insert("DEFAULT".to_string(), TokenType::Default);
        keywords.insert("GENERATED".to_string(), TokenType::Generated);
        keywords.insert("ALWAYS".to_string(), TokenType::Always);
        keywords.insert("IDENTITY".to_string(), TokenType::Identity);

        // Date/Time keywords
        keywords.insert("INTERVAL".to_string(), TokenType::Interval);

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
        keywords.insert("LEARN".to_string(), TokenType::Learn);
        keywords.insert("ADAPT".to_string(), TokenType::Adapt);
        keywords.insert("EPOCHS".to_string(), TokenType::Epochs);
        keywords.insert("ALGORITHM".to_string(), TokenType::Algorithm);
        keywords.insert(
            "TRAINING_DATA".to_string(),
            TokenType::Identifier("TRAINING_DATA".to_string()),
        );
        keywords.insert(
            "FEATURES".to_string(),
            TokenType::Identifier("FEATURES".to_string()),
        );
        keywords.insert("LEARNING_RATE".to_string(), TokenType::LearningRate);
        keywords.insert(
            "RULE".to_string(),
            TokenType::Identifier("RULE".to_string()),
        );

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

        // Transaction control keywords
        keywords.insert("BEGIN".to_string(), TokenType::Begin);
        keywords.insert("START".to_string(), TokenType::Start);
        keywords.insert("TRANSACTION".to_string(), TokenType::Transaction);
        keywords.insert("COMMIT".to_string(), TokenType::Commit);
        keywords.insert("ROLLBACK".to_string(), TokenType::Rollback);
        keywords.insert("SAVEPOINT".to_string(), TokenType::Savepoint);
        keywords.insert("RELEASE".to_string(), TokenType::Release);
        keywords.insert("TO".to_string(), TokenType::To);
    }

    /// Initialize operator mappings with precedence for Pratt parsing
    fn initialize_operators(operators: &mut HashMap<String, OperatorInfo>) {
        // Logical operators (lowest precedence)
        operators.insert(
            "OR".to_string(),
            OperatorInfo {
                operator: BinaryOperator::Or,
                precedence: Precedence::Or,
                right_associative: false,
            },
        );
        operators.insert(
            "AND".to_string(),
            OperatorInfo {
                operator: BinaryOperator::And,
                precedence: Precedence::And,
                right_associative: false,
            },
        );

        // Comparison operators
        operators.insert(
            "=".to_string(),
            OperatorInfo {
                operator: BinaryOperator::Equal,
                precedence: Precedence::Comparison,
                right_associative: false,
            },
        );
        operators.insert(
            "!=".to_string(),
            OperatorInfo {
                operator: BinaryOperator::NotEqual,
                precedence: Precedence::Comparison,
                right_associative: false,
            },
        );
        operators.insert(
            "<>".to_string(),
            OperatorInfo {
                operator: BinaryOperator::NotEqual,
                precedence: Precedence::Comparison,
                right_associative: false,
            },
        );
        operators.insert(
            "<".to_string(),
            OperatorInfo {
                operator: BinaryOperator::LessThan,
                precedence: Precedence::Comparison,
                right_associative: false,
            },
        );
        operators.insert(
            "<=".to_string(),
            OperatorInfo {
                operator: BinaryOperator::LessThanOrEqual,
                precedence: Precedence::Comparison,
                right_associative: false,
            },
        );
        operators.insert(
            ">".to_string(),
            OperatorInfo {
                operator: BinaryOperator::GreaterThan,
                precedence: Precedence::Comparison,
                right_associative: false,
            },
        );
        operators.insert(
            ">=".to_string(),
            OperatorInfo {
                operator: BinaryOperator::GreaterThanOrEqual,
                precedence: Precedence::Comparison,
                right_associative: false,
            },
        );
        operators.insert(
            "LIKE".to_string(),
            OperatorInfo {
                operator: BinaryOperator::Like,
                precedence: Precedence::Comparison,
                right_associative: false,
            },
        );
        operators.insert(
            "IN".to_string(),
            OperatorInfo {
                operator: BinaryOperator::In,
                precedence: Precedence::Comparison,
                right_associative: false,
            },
        );

        // Additive operators
        operators.insert(
            "+".to_string(),
            OperatorInfo {
                operator: BinaryOperator::Add,
                precedence: Precedence::Additive,
                right_associative: false,
            },
        );
        operators.insert(
            "-".to_string(),
            OperatorInfo {
                operator: BinaryOperator::Subtract,
                precedence: Precedence::Additive,
                right_associative: false,
            },
        );

        // Multiplicative operators (higher precedence than additive)
        operators.insert(
            "*".to_string(),
            OperatorInfo {
                operator: BinaryOperator::Multiply,
                precedence: Precedence::Multiplicative,
                right_associative: false,
            },
        );
        operators.insert(
            "/".to_string(),
            OperatorInfo {
                operator: BinaryOperator::Divide,
                precedence: Precedence::Multiplicative,
                right_associative: false,
            },
        );
        operators.insert(
            "%".to_string(),
            OperatorInfo {
                operator: BinaryOperator::Modulo,
                precedence: Precedence::Multiplicative,
                right_associative: false,
            },
        );

        // Neuromorphic operators
        operators.insert(
            "SYNAPTIC_SIMILAR".to_string(),
            OperatorInfo {
                operator: BinaryOperator::SynapticSimilarity,
                precedence: Precedence::Neuromorphic,
                right_associative: false,
            },
        );
        operators.insert(
            "HEBBIAN_STRENGTHEN".to_string(),
            OperatorInfo {
                operator: BinaryOperator::HebbianStrengthening,
                precedence: Precedence::Neuromorphic,
                right_associative: false,
            },
        );
        operators.insert(
            "PLASTICITY_UPDATE".to_string(),
            OperatorInfo {
                operator: BinaryOperator::PlasticityUpdate,
                precedence: Precedence::Neuromorphic,
                right_associative: false,
            },
        );

        // Quantum operators
        operators.insert(
            "ENTANGLE".to_string(),
            OperatorInfo {
                operator: BinaryOperator::QuantumEntanglement,
                precedence: Precedence::Quantum,
                right_associative: false,
            },
        );
        operators.insert(
            "SUPERPOSITION_COLLAPSE".to_string(),
            OperatorInfo {
                operator: BinaryOperator::SuperpositionCollapse,
                precedence: Precedence::Quantum,
                right_associative: false,
            },
        );
        operators.insert(
            "AMPLITUDE_INTERFERE".to_string(),
            OperatorInfo {
                operator: BinaryOperator::AmplitudeInterference,
                precedence: Precedence::Quantum,
                right_associative: false,
            },
        );
    }

    /// Validate AST structure
    fn validate_ast(&self, _ast: &Statement) -> QSQLResult<()> {
        // Basic validation - could be expanded
        Ok(())
    }

    /// Parse expression using Pratt parsing (operator precedence parsing)
    ///
    /// This implements a Pratt parser which correctly handles operator precedence
    /// and associativity. The algorithm works by:
    /// 1. Parsing a primary expression (literals, identifiers, parentheses)
    /// 2. While the next operator has higher precedence than min_precedence:
    ///    - Parse the operator
    ///    - Recursively parse the right side with appropriate precedence
    ///    - Build the binary expression
    fn parse_expression(&self, tokens: &[TokenType], i: &mut usize) -> QSQLResult<Expression> {
        self.parse_expression_with_precedence(tokens, i, Precedence::None)
    }

    /// Parse expression with minimum precedence (core of Pratt parser)
    fn parse_expression_with_precedence(
        &self,
        tokens: &[TokenType],
        i: &mut usize,
        min_precedence: Precedence,
    ) -> QSQLResult<Expression> {
        // First, parse the left-hand side (prefix expression)
        let mut left = self.parse_prefix_expression(tokens, i)?;

        // Then, handle infix operators using precedence climbing
        while *i < tokens.len() {
            // Check for IS NULL or IS NOT NULL
            if matches!(tokens[*i], TokenType::Is) {
                *i += 1; // consume IS
                let negated = if *i < tokens.len() && matches!(tokens[*i], TokenType::Not) {
                    *i += 1; // consume NOT
                    true
                } else {
                    false
                };

                // Expect NULL
                if *i < tokens.len() && matches!(tokens[*i], TokenType::Null) {
                    *i += 1; // consume NULL
                    left = Expression::IsNull {
                        expr: Box::new(left),
                        negated,
                    };
                    continue;
                } else {
                    return Err(QSQLError::ParseError {
                        message: "Expected NULL after IS".to_string(),
                        position: *i,
                    });
                }
            }

            // Check for NOT IN (two-token sequence)
            if matches!(tokens[*i], TokenType::Not)
                && *i + 1 < tokens.len()
                && matches!(tokens[*i + 1], TokenType::In)
            {
                *i += 2; // consume NOT IN
                left = self.parse_in_list(tokens, i, left, true)?;
                continue;
            }

            // Check for IN operator - special handling for IN (list)
            if matches!(tokens[*i], TokenType::In) {
                *i += 1; // consume IN
                left = self.parse_in_list(tokens, i, left, false)?;
                continue;
            }

            // Get operator info for current token
            let op_info = match self.get_operator_info(&tokens[*i]) {
                Some(info) => info,
                None => break, // Not an operator, stop parsing
            };

            // If operator precedence is too low, stop
            if op_info.precedence < min_precedence {
                break;
            }

            // Consume the operator token
            *i += 1;

            // Determine the precedence for the right side
            // For left-associative: use next higher precedence
            // For right-associative: use same precedence
            let next_min_precedence = if op_info.right_associative {
                op_info.precedence
            } else {
                op_info.precedence.next()
            };

            // Parse the right-hand side recursively
            let right = self.parse_expression_with_precedence(tokens, i, next_min_precedence)?;

            // Build the binary expression
            left = Expression::BinaryOp {
                left: Box::new(left),
                operator: op_info.operator,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// Parse IN list expression: IN (value1, value2, ...) or NOT IN (value1, value2, ...)
    fn parse_in_list(
        &self,
        tokens: &[TokenType],
        i: &mut usize,
        left: Expression,
        negated: bool,
    ) -> QSQLResult<Expression> {
        // Expect opening parenthesis
        if *i >= tokens.len() || !matches!(tokens[*i], TokenType::LeftParen) {
            return Err(QSQLError::ParseError {
                message: "Expected '(' after IN".to_string(),
                position: *i,
            });
        }
        *i += 1; // consume '('

        // Check if this is a subquery (starts with SELECT)
        if *i < tokens.len() && matches!(tokens[*i], TokenType::Select) {
            // Parse the subquery
            let subquery = self.parse_select_statement_at(tokens, i)?;

            // Expect closing paren
            if *i >= tokens.len() || !matches!(tokens[*i], TokenType::RightParen) {
                return Err(QSQLError::ParseError {
                    message: "Expected ')' after subquery in IN clause".to_string(),
                    position: *i,
                });
            }
            *i += 1; // consume ')'

            return Ok(Expression::InSubquery {
                expr: Box::new(left),
                subquery: Box::new(subquery),
                negated,
            });
        }

        let mut list = Vec::new();

        // Parse the list of values
        loop {
            if *i >= tokens.len() {
                return Err(QSQLError::ParseError {
                    message: "Unclosed IN list".to_string(),
                    position: *i,
                });
            }

            // Check for closing paren (empty list is allowed)
            if matches!(tokens[*i], TokenType::RightParen) {
                *i += 1; // consume ')'
                break;
            }

            // Parse the next value in the list
            let value = self.parse_prefix_expression(tokens, i)?;
            list.push(value);

            // Check for comma or closing paren
            if *i < tokens.len() && matches!(tokens[*i], TokenType::Comma) {
                *i += 1; // consume ','
            } else if *i < tokens.len() && matches!(tokens[*i], TokenType::RightParen) {
                *i += 1; // consume ')'
                break;
            } else if *i >= tokens.len() {
                return Err(QSQLError::ParseError {
                    message: "Expected ',' or ')' in IN list".to_string(),
                    position: *i,
                });
            }
        }

        Ok(Expression::InList {
            expr: Box::new(left),
            list,
            negated,
        })
    }

    /// Parse CASE expression: CASE WHEN condition THEN result [WHEN ...] [ELSE result] END
    fn parse_case_expression(&self, tokens: &[TokenType], i: &mut usize) -> QSQLResult<Expression> {
        // Consume CASE token
        *i += 1;

        let mut when_clauses = Vec::new();
        let mut else_result = None;

        // Parse WHEN clauses
        loop {
            if *i >= tokens.len() {
                return Err(QSQLError::ParseError {
                    message: "Unexpected end of CASE expression".to_string(),
                    position: *i,
                });
            }

            match &tokens[*i] {
                TokenType::When => {
                    *i += 1; // consume WHEN

                    // Parse condition
                    let condition =
                        self.parse_expression_with_precedence(tokens, i, Precedence::None)?;

                    // Expect THEN
                    if *i >= tokens.len() || !matches!(tokens[*i], TokenType::Then) {
                        return Err(QSQLError::ParseError {
                            message: "Expected THEN after WHEN condition".to_string(),
                            position: *i,
                        });
                    }
                    *i += 1; // consume THEN

                    // Parse result expression
                    let result =
                        self.parse_expression_with_precedence(tokens, i, Precedence::None)?;

                    when_clauses.push((Box::new(condition), Box::new(result)));
                }
                TokenType::Else => {
                    *i += 1; // consume ELSE

                    // Parse else result
                    let result =
                        self.parse_expression_with_precedence(tokens, i, Precedence::None)?;
                    else_result = Some(Box::new(result));

                    // After ELSE, we expect END
                    if *i >= tokens.len() || !matches!(tokens[*i], TokenType::End) {
                        return Err(QSQLError::ParseError {
                            message: "Expected END after ELSE clause".to_string(),
                            position: *i,
                        });
                    }
                    *i += 1; // consume END
                    break;
                }
                TokenType::End => {
                    *i += 1; // consume END
                    break;
                }
                _ => {
                    return Err(QSQLError::ParseError {
                        message: format!(
                            "Expected WHEN, ELSE, or END in CASE expression, found {:?}",
                            tokens[*i]
                        ),
                        position: *i,
                    });
                }
            }
        }

        // Must have at least one WHEN clause
        if when_clauses.is_empty() {
            return Err(QSQLError::ParseError {
                message: "CASE expression must have at least one WHEN clause".to_string(),
                position: *i,
            });
        }

        Ok(Expression::Case {
            when_clauses,
            else_result,
        })
    }

    /// Parse EXTRACT expression: EXTRACT(field FROM source)
    fn parse_extract_expression(
        &self,
        tokens: &[TokenType],
        i: &mut usize,
    ) -> QSQLResult<Expression> {
        // Consume EXTRACT token
        *i += 1;

        // Expect opening parenthesis
        if *i >= tokens.len() || !matches!(tokens[*i], TokenType::LeftParen) {
            return Err(QSQLError::ParseError {
                message: "Expected '(' after EXTRACT".to_string(),
                position: *i,
            });
        }
        *i += 1; // consume '('

        // Parse the field identifier (YEAR, MONTH, DAY, etc.)
        let field = match &tokens[*i] {
            TokenType::Identifier(name) => name.to_uppercase(),
            _ => {
                return Err(QSQLError::ParseError {
                    message: format!(
                        "Expected field identifier in EXTRACT expression, found {:?}",
                        tokens[*i]
                    ),
                    position: *i,
                });
            }
        };
        *i += 1; // consume field identifier

        // Expect FROM keyword
        if *i >= tokens.len() || !matches!(tokens[*i], TokenType::From) {
            return Err(QSQLError::ParseError {
                message: "Expected FROM keyword in EXTRACT expression".to_string(),
                position: *i,
            });
        }
        *i += 1; // consume FROM

        // Parse source expression
        let source = self.parse_expression_with_precedence(tokens, i, Precedence::None)?;

        // Expect closing parenthesis
        if *i >= tokens.len() || !matches!(tokens[*i], TokenType::RightParen) {
            return Err(QSQLError::ParseError {
                message: "Expected ')' at end of EXTRACT expression".to_string(),
                position: *i,
            });
        }
        *i += 1; // consume ')'

        Ok(Expression::Extract {
            field,
            source: Box::new(source),
        })
    }

    /// Parse prefix expression (primary expressions and unary operators)
    fn parse_prefix_expression(
        &self,
        tokens: &[TokenType],
        i: &mut usize,
    ) -> QSQLResult<Expression> {
        if *i >= tokens.len() {
            return Err(QSQLError::ParseError {
                message: "Unexpected end of expression".to_string(),
                position: *i,
            });
        }

        match &tokens[*i] {
            // CASE expression: CASE WHEN ... THEN ... [ELSE ...] END
            TokenType::Case => self.parse_case_expression(tokens, i),

            // EXTRACT expression: EXTRACT(field FROM source)
            TokenType::Extract => self.parse_extract_expression(tokens, i),

            // Unary NOT operator
            TokenType::Not => {
                *i += 1;
                let operand = self.parse_expression_with_precedence(tokens, i, Precedence::Not)?;
                Ok(Expression::UnaryOp {
                    operator: UnaryOperator::Not,
                    operand: Box::new(operand),
                })
            }

            // Unary minus
            TokenType::Minus => {
                *i += 1;
                let operand =
                    self.parse_expression_with_precedence(tokens, i, Precedence::Unary)?;
                Ok(Expression::UnaryOp {
                    operator: UnaryOperator::Minus,
                    operand: Box::new(operand),
                })
            }

            // Unary plus (usually a no-op, but we support it)
            TokenType::Plus => {
                *i += 1;
                let operand =
                    self.parse_expression_with_precedence(tokens, i, Precedence::Unary)?;
                Ok(Expression::UnaryOp {
                    operator: UnaryOperator::Plus,
                    operand: Box::new(operand),
                })
            }

            // Parenthesized expression
            TokenType::LeftParen => {
                *i += 1;
                let expr = self.parse_expression_with_precedence(tokens, i, Precedence::None)?;

                if *i >= tokens.len() || !matches!(tokens[*i], TokenType::RightParen) {
                    return Err(QSQLError::ParseError {
                        message: "Expected closing parenthesis".to_string(),
                        position: *i,
                    });
                }
                *i += 1; // consume ')'
                Ok(expr)
            }

            // Literals
            TokenType::StringLiteral(s) => {
                *i += 1;
                Ok(Expression::Literal(Literal::String(s.clone())))
            }
            TokenType::IntegerLiteral(n) => {
                *i += 1;
                Ok(Expression::Literal(Literal::Integer(*n)))
            }
            TokenType::FloatLiteral(f) => {
                *i += 1;
                Ok(Expression::Literal(Literal::Float(*f)))
            }
            TokenType::BooleanLiteral(b) => {
                *i += 1;
                Ok(Expression::Literal(Literal::Boolean(*b)))
            }
            TokenType::DNALiteral(dna) => {
                *i += 1;
                Ok(Expression::Literal(Literal::DNA(dna.clone())))
            }
            TokenType::Null => {
                *i += 1;
                Ok(Expression::Literal(Literal::Null))
            }

            // Identifier or function call (including qualified names like table.column)
            TokenType::Identifier(name) => {
                let mut full_name = name.clone();
                *i += 1;

                // Check for qualified name (e.g., u.id, table.column)
                while *i + 1 < tokens.len() && matches!(tokens[*i], TokenType::Dot) {
                    if let TokenType::Identifier(next_part) = &tokens[*i + 1] {
                        full_name.push('.');
                        full_name.push_str(next_part);
                        *i += 2; // consume '.' and identifier
                    } else {
                        break;
                    }
                }

                // Check if this is a function call
                if *i < tokens.len() && matches!(tokens[*i], TokenType::LeftParen) {
                    self.parse_function_call(tokens, i, full_name)
                } else {
                    Ok(Expression::Identifier(full_name))
                }
            }

            // NULL keyword (handled separately from literal)
            TokenType::Is => {
                // "IS NULL" or "IS NOT NULL" are handled as infix operators
                Err(QSQLError::ParseError {
                    message: "IS must follow an expression".to_string(),
                    position: *i,
                })
            }

            _ => Err(QSQLError::ParseError {
                message: format!("Unexpected token in expression: {:?}", tokens[*i]),
                position: *i,
            }),
        }
    }

    /// Parse function call expression
    fn parse_function_call(
        &self,
        tokens: &[TokenType],
        i: &mut usize,
        function_name: String,
    ) -> QSQLResult<Expression> {
        // Consume '('
        *i += 1;

        let mut args = Vec::new();

        // Parse arguments
        loop {
            if *i >= tokens.len() {
                return Err(QSQLError::ParseError {
                    message: "Unclosed function call".to_string(),
                    position: *i,
                });
            }

            // Check for closing paren
            if matches!(tokens[*i], TokenType::RightParen) {
                *i += 1;
                break;
            }

            // Special handling for COUNT(*) - treat * as a special argument
            if matches!(tokens[*i], TokenType::Multiply) {
                *i += 1;
                args.push(Expression::Literal(Literal::String("*".to_string())));
                // Check for closing paren after *
                if *i < tokens.len() && matches!(tokens[*i], TokenType::RightParen) {
                    *i += 1;
                    break;
                }
                continue;
            }

            // Check for DISTINCT keyword (for COUNT(DISTINCT column))
            if matches!(tokens[*i], TokenType::Distinct) {
                *i += 1;
                // Parse the column after DISTINCT
                let arg = self.parse_expression_with_precedence(tokens, i, Precedence::None)?;
                // Wrap it to indicate DISTINCT
                if let Expression::Identifier(col) = arg {
                    args.push(Expression::Identifier(format!("DISTINCT {}", col)));
                } else {
                    args.push(arg);
                }
            } else if matches!(tokens[*i], TokenType::Interval) {
                // Special handling for INTERVAL syntax (DATE_ADD/DATE_SUB)
                // INTERVAL expr unit -> encoded as string "expr unit"
                *i += 1; // consume INTERVAL

                // Parse the interval expression (could be number or expression)
                let expr = self.parse_expression_with_precedence(tokens, i, Precedence::None)?;

                // Parse the unit (DAY, MONTH, YEAR, HOUR, etc.)
                if *i >= tokens.len() {
                    return Err(QSQLError::ParseError {
                        message: "Expected time unit after INTERVAL expression".to_string(),
                        position: *i,
                    });
                }

                let unit = match &tokens[*i] {
                    TokenType::Identifier(s) => s.to_uppercase(),
                    _ => {
                        return Err(QSQLError::ParseError {
                            message: "Expected time unit (DAY, MONTH, YEAR, etc.) after INTERVAL expression".to_string(),
                            position: *i,
                        });
                    }
                };
                *i += 1; // consume unit

                // Encode INTERVAL as a special string: "INTERVAL:<expr>:<unit>"
                // We'll extract the expression value at execution time
                args.push(expr);
                args.push(Expression::Literal(Literal::String(format!(
                    "INTERVAL_UNIT:{}",
                    unit
                ))));
            } else {
                // Parse argument expression
                let arg = self.parse_expression_with_precedence(tokens, i, Precedence::None)?;
                args.push(arg);
            }

            // Check for comma or closing paren
            if *i < tokens.len() {
                if matches!(tokens[*i], TokenType::Comma) {
                    *i += 1;
                } else if !matches!(tokens[*i], TokenType::RightParen) {
                    return Err(QSQLError::ParseError {
                        message: "Expected ',' or ')' in function call".to_string(),
                        position: *i,
                    });
                }
            }
        }

        Ok(Expression::FunctionCall {
            name: function_name,
            args,
        })
    }

    /// Get operator info from token for Pratt parsing
    fn get_operator_info(&self, token: &TokenType) -> Option<OperatorInfo> {
        let op_key = match token {
            // Arithmetic operators
            TokenType::Plus => "+",
            TokenType::Minus => "-",
            TokenType::Multiply => "*",
            TokenType::Divide => "/",
            TokenType::Modulo => "%",

            // Comparison operators
            TokenType::Equal => "=",
            TokenType::NotEqual => "!=",
            TokenType::LessThan => "<",
            TokenType::LessThanOrEqual => "<=",
            TokenType::GreaterThan => ">",
            TokenType::GreaterThanOrEqual => ">=",

            // Logical operators
            TokenType::And => "AND",
            TokenType::Or => "OR",

            // String operators
            TokenType::Like => "LIKE",
            TokenType::In => "IN",

            // Neuromorphic operators (from keywords)
            TokenType::Identifier(name) => {
                let upper = name.to_uppercase();
                if self.operators.contains_key(&upper) {
                    return self.operators.get(&upper).cloned();
                }
                return None;
            }

            // Not an infix operator
            _ => return None,
        };

        self.operators.get(op_key).cloned()
    }
}

impl Default for QSQLParser {
    fn default() -> Self {
        match Self::with_config(ParserConfig::default()) {
            Ok(parser) => parser,
            Err(_) => {
                // Fallback to a minimal parser if creation fails
                QSQLParser {
                    config: ParserConfig::default(),
                    natural_language_processor: None,
                    keywords: HashMap::new(),
                    operators: HashMap::new(),
                }
            }
        }
    }
}

/// Simple natural language processor for demo purposes
use crate::natural_language::NaturalLanguageProcessor;
