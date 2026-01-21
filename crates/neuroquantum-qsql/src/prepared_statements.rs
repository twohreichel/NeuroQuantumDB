//! Prepared Statement Management for QSQL
//!
//! This module provides session-scoped prepared statement storage and execution
//! with query plan caching for improved performance.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::ast::{
    DeallocateStatement, ExecuteStatement, Expression, ParameterRef, PrepareStatement, Statement,
};
use crate::error::{QSQLError, QSQLResult};
use crate::query_plan::QueryPlan;

/// A prepared statement with its cached query plan
#[derive(Debug, Clone)]
pub struct PreparedStatement {
    /// Name of the prepared statement
    pub name: String,
    /// The original SQL statement (with parameter placeholders)
    pub statement: Statement,
    /// Cached query plan (if available)
    pub cached_plan: Option<Arc<QueryPlan>>,
    /// Number of parameters expected
    pub parameter_count: usize,
    /// Parameter names for named parameters
    pub parameter_names: Vec<String>,
    /// Statistics for this prepared statement
    pub stats: PreparedStatementStats,
    /// When this statement was prepared
    pub created_at: Instant,
}

/// Statistics for a prepared statement
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PreparedStatementStats {
    /// Number of times this statement has been executed
    pub execution_count: u64,
    /// Total execution time across all executions
    pub total_execution_time: Duration,
    /// Average execution time
    pub average_execution_time: Duration,
    /// Cache hits (when cached plan was used)
    pub cache_hits: u64,
}

/// Session-scoped prepared statement manager
///
/// Each session/connection should have its own `PreparedStatementManager`
/// for proper isolation of prepared statements.
#[derive(Debug, Default)]
pub struct PreparedStatementManager {
    /// Map of statement name to prepared statement
    statements: HashMap<String, PreparedStatement>,
    /// Maximum number of prepared statements allowed per session
    max_statements: usize,
    /// Total number of executions across all statements
    total_executions: u64,
}

impl PreparedStatementManager {
    /// Create a new `PreparedStatementManager` with default settings
    #[must_use]
    pub fn new() -> Self {
        Self {
            statements: HashMap::new(),
            max_statements: 1000, // Default limit
            total_executions: 0,
        }
    }

    /// Create a new `PreparedStatementManager` with a custom statement limit
    #[must_use]
    pub fn with_max_statements(max_statements: usize) -> Self {
        Self {
            statements: HashMap::new(),
            max_statements,
            total_executions: 0,
        }
    }

    /// Prepare a statement and store it
    ///
    /// Returns an error if a statement with the same name already exists
    /// or if the maximum number of statements has been reached.
    pub fn prepare(&mut self, prepare_stmt: &PrepareStatement) -> QSQLResult<()> {
        // Check if statement already exists
        if self.statements.contains_key(&prepare_stmt.name) {
            return Err(QSQLError::PreparedStatementError {
                message: format!(
                    "Prepared statement '{}' already exists. Use DEALLOCATE first.",
                    prepare_stmt.name
                ),
            });
        }

        // Check statement limit
        if self.statements.len() >= self.max_statements {
            return Err(QSQLError::PreparedStatementError {
                message: format!(
                    "Maximum number of prepared statements ({}) reached",
                    self.max_statements
                ),
            });
        }

        // Count parameters in the statement
        let (param_count, param_names) = count_parameters(&prepare_stmt.statement);

        let prepared = PreparedStatement {
            name: prepare_stmt.name.clone(),
            statement: (*prepare_stmt.statement).clone(),
            cached_plan: None,
            parameter_count: param_count,
            parameter_names: param_names,
            stats: PreparedStatementStats::default(),
            created_at: Instant::now(),
        };

        info!(
            "Prepared statement '{}' with {} parameters",
            prepare_stmt.name, param_count
        );

        self.statements.insert(prepare_stmt.name.clone(), prepared);
        Ok(())
    }

    /// Get a prepared statement by name
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&PreparedStatement> {
        self.statements.get(name)
    }

    /// Get a mutable reference to a prepared statement by name
    pub fn get_mut(&mut self, name: &str) -> Option<&mut PreparedStatement> {
        self.statements.get_mut(name)
    }

    /// Bind parameters to a prepared statement and return the bound statement
    ///
    /// This replaces parameter placeholders with actual values.
    pub fn bind_parameters(
        &self,
        name: &str,
        execute_stmt: &ExecuteStatement,
    ) -> QSQLResult<Statement> {
        let prepared =
            self.statements
                .get(name)
                .ok_or_else(|| QSQLError::PreparedStatementError {
                    message: format!("Prepared statement '{name}' not found"),
                })?;

        // Build parameter map from positional and named parameters
        let mut param_values: HashMap<ParameterRef, Expression> = HashMap::new();

        // Add positional parameters ($1, $2, etc.)
        for (idx, value) in execute_stmt.parameters.iter().enumerate() {
            param_values.insert(ParameterRef::Positional((idx + 1) as u32), value.clone());
        }

        // Add named parameters (:name)
        for (name, value) in &execute_stmt.named_parameters {
            param_values.insert(ParameterRef::Named(name.clone()), value.clone());
        }

        // Check that we have all required parameters
        let provided_count = execute_stmt.parameters.len() + execute_stmt.named_parameters.len();
        if provided_count < prepared.parameter_count {
            return Err(QSQLError::PreparedStatementError {
                message: format!(
                    "Prepared statement '{}' requires {} parameters, but {} were provided",
                    name, prepared.parameter_count, provided_count
                ),
            });
        }

        // Clone the statement and substitute parameters
        let bound_statement = substitute_parameters(&prepared.statement, &param_values)?;

        debug!(
            "Bound {} parameters to prepared statement '{}'",
            provided_count, name
        );

        Ok(bound_statement)
    }

    /// Record execution statistics for a prepared statement
    pub fn record_execution(&mut self, name: &str, execution_time: Duration) {
        if let Some(prepared) = self.statements.get_mut(name) {
            prepared.stats.execution_count += 1;
            prepared.stats.total_execution_time += execution_time;
            prepared.stats.average_execution_time = Duration::from_nanos(
                (prepared.stats.total_execution_time.as_nanos()
                    / u128::from(prepared.stats.execution_count)) as u64,
            );
            self.total_executions += 1;
        }
    }

    /// Deallocate a prepared statement
    pub fn deallocate(&mut self, name: &str) -> QSQLResult<()> {
        if self.statements.remove(name).is_some() {
            info!("Deallocated prepared statement '{}'", name);
            Ok(())
        } else {
            Err(QSQLError::PreparedStatementError {
                message: format!("Prepared statement '{name}' not found"),
            })
        }
    }

    /// Deallocate all prepared statements
    pub fn deallocate_all(&mut self) {
        let count = self.statements.len();
        self.statements.clear();
        info!("Deallocated {} prepared statements", count);
    }

    /// Handle a DEALLOCATE statement
    pub fn handle_deallocate(&mut self, deallocate_stmt: &DeallocateStatement) -> QSQLResult<()> {
        match &deallocate_stmt.name {
            | Some(name) => self.deallocate(name),
            | None => {
                self.deallocate_all();
                Ok(())
            },
        }
    }

    /// List all prepared statement names
    #[must_use]
    pub fn list_statements(&self) -> Vec<&str> {
        self.statements
            .keys()
            .map(std::string::String::as_str)
            .collect()
    }

    /// Get statistics for all prepared statements
    #[must_use]
    pub fn get_statistics(&self) -> PreparedStatementsStatistics {
        PreparedStatementsStatistics {
            total_statements: self.statements.len(),
            total_executions: self.total_executions,
            statements: self
                .statements
                .iter()
                .map(|(name, stmt)| (name.clone(), stmt.stats.clone()))
                .collect(),
        }
    }

    /// Check if a statement with the given name exists
    #[must_use]
    pub fn exists(&self, name: &str) -> bool {
        self.statements.contains_key(name)
    }
}

/// Statistics for all prepared statements in a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreparedStatementsStatistics {
    /// Total number of prepared statements
    pub total_statements: usize,
    /// Total number of executions across all statements
    pub total_executions: u64,
    /// Per-statement statistics
    pub statements: HashMap<String, PreparedStatementStats>,
}

/// Count the number of parameters in a statement and collect parameter names
fn count_parameters(statement: &Statement) -> (usize, Vec<String>) {
    let mut positional_max = 0u32;
    let mut named_params = Vec::new();

    fn visit_expression(
        expr: &Expression,
        positional_max: &mut u32,
        named_params: &mut Vec<String>,
    ) {
        match expr {
            | Expression::Parameter(ParameterRef::Positional(idx)) => {
                if *idx > *positional_max {
                    *positional_max = *idx;
                }
            },
            | Expression::Parameter(ParameterRef::Named(name)) => {
                if !named_params.contains(name) {
                    named_params.push(name.clone());
                }
            },
            | Expression::BinaryOp { left, right, .. } => {
                visit_expression(left, positional_max, named_params);
                visit_expression(right, positional_max, named_params);
            },
            | Expression::UnaryOp { operand, .. } => {
                visit_expression(operand, positional_max, named_params);
            },
            | Expression::FunctionCall { args, .. } => {
                for arg in args {
                    visit_expression(arg, positional_max, named_params);
                }
            },
            | Expression::InList { expr, list, .. } => {
                visit_expression(expr, positional_max, named_params);
                for item in list {
                    visit_expression(item, positional_max, named_params);
                }
            },
            | Expression::Case {
                when_clauses,
                else_result,
            } => {
                for (cond, result) in when_clauses {
                    visit_expression(cond, positional_max, named_params);
                    visit_expression(result, positional_max, named_params);
                }
                if let Some(else_expr) = else_result {
                    visit_expression(else_expr, positional_max, named_params);
                }
            },
            | _ => {},
        }
    }

    fn visit_statement(stmt: &Statement, positional_max: &mut u32, named_params: &mut Vec<String>) {
        match stmt {
            | Statement::Select(select) => {
                if let Some(where_clause) = &select.where_clause {
                    visit_expression(where_clause, positional_max, named_params);
                }
                if let Some(having) = &select.having {
                    visit_expression(having, positional_max, named_params);
                }
                for item in &select.select_list {
                    if let crate::ast::SelectItem::Expression { expr, .. } = item {
                        visit_expression(expr, positional_max, named_params);
                    }
                }
            },
            | Statement::Insert(insert) => {
                for row in &insert.values {
                    for expr in row {
                        visit_expression(expr, positional_max, named_params);
                    }
                }
            },
            | Statement::Update(update) => {
                for assignment in &update.assignments {
                    visit_expression(&assignment.value, positional_max, named_params);
                }
                if let Some(where_clause) = &update.where_clause {
                    visit_expression(where_clause, positional_max, named_params);
                }
            },
            | Statement::Delete(delete) => {
                if let Some(where_clause) = &delete.where_clause {
                    visit_expression(where_clause, positional_max, named_params);
                }
            },
            | _ => {},
        }
    }

    visit_statement(statement, &mut positional_max, &mut named_params);

    let total_count = positional_max as usize + named_params.len();
    (total_count, named_params)
}

/// Substitute parameters in a statement with actual values
fn substitute_parameters(
    statement: &Statement,
    params: &HashMap<ParameterRef, Expression>,
) -> QSQLResult<Statement> {
    fn substitute_expression(
        expr: &Expression,
        params: &HashMap<ParameterRef, Expression>,
    ) -> QSQLResult<Expression> {
        match expr {
            | Expression::Parameter(param_ref) => params.get(param_ref).cloned().ok_or_else(|| {
                let param_str = match param_ref {
                    | ParameterRef::Positional(idx) => format!("${idx}"),
                    | ParameterRef::Named(name) => format!(":{name}"),
                };
                QSQLError::PreparedStatementError {
                    message: format!("Missing parameter value for {param_str}"),
                }
            }),
            | Expression::BinaryOp {
                left,
                operator,
                right,
            } => Ok(Expression::BinaryOp {
                left: Box::new(substitute_expression(left, params)?),
                operator: operator.clone(),
                right: Box::new(substitute_expression(right, params)?),
            }),
            | Expression::UnaryOp { operator, operand } => Ok(Expression::UnaryOp {
                operator: operator.clone(),
                operand: Box::new(substitute_expression(operand, params)?),
            }),
            | Expression::FunctionCall { name, args } => {
                let new_args: Result<Vec<_>, _> = args
                    .iter()
                    .map(|arg| substitute_expression(arg, params))
                    .collect();
                Ok(Expression::FunctionCall {
                    name: name.clone(),
                    args: new_args?,
                })
            },
            | Expression::InList {
                expr,
                list,
                negated,
            } => {
                let new_list: Result<Vec<_>, _> = list
                    .iter()
                    .map(|item| substitute_expression(item, params))
                    .collect();
                Ok(Expression::InList {
                    expr: Box::new(substitute_expression(expr, params)?),
                    list: new_list?,
                    negated: *negated,
                })
            },
            | Expression::Case {
                when_clauses,
                else_result,
            } => {
                let new_when: QSQLResult<Vec<_>> = when_clauses
                    .iter()
                    .map(|(cond, result)| {
                        let new_cond = substitute_expression(cond, params)?;
                        let new_result = substitute_expression(result, params)?;
                        Ok((Box::new(new_cond), Box::new(new_result)))
                    })
                    .collect();
                let new_else = else_result
                    .as_ref()
                    .map(|e| substitute_expression(e, params).map(Box::new))
                    .transpose()?;
                Ok(Expression::Case {
                    when_clauses: new_when?,
                    else_result: new_else,
                })
            },
            // Other expression types are returned as-is
            | other => Ok(other.clone()),
        }
    }

    fn substitute_statement(
        stmt: &Statement,
        params: &HashMap<ParameterRef, Expression>,
    ) -> QSQLResult<Statement> {
        match stmt {
            | Statement::Select(select) => {
                let mut new_select = select.clone();
                if let Some(where_clause) = &select.where_clause {
                    new_select.where_clause = Some(substitute_expression(where_clause, params)?);
                }
                if let Some(having) = &select.having {
                    new_select.having = Some(substitute_expression(having, params)?);
                }
                // Substitute in select list
                new_select.select_list = select
                    .select_list
                    .iter()
                    .map(|item| match item {
                        | crate::ast::SelectItem::Expression { expr, alias } => {
                            Ok(crate::ast::SelectItem::Expression {
                                expr: substitute_expression(expr, params)?,
                                alias: alias.clone(),
                            })
                        },
                        | other => Ok(other.clone()),
                    })
                    .collect::<QSQLResult<Vec<_>>>()?;
                Ok(Statement::Select(new_select))
            },
            | Statement::Insert(insert) => {
                let mut new_insert = insert.clone();
                new_insert.values = insert
                    .values
                    .iter()
                    .map(|row| {
                        row.iter()
                            .map(|expr| substitute_expression(expr, params))
                            .collect::<QSQLResult<Vec<_>>>()
                    })
                    .collect::<QSQLResult<Vec<_>>>()?;
                Ok(Statement::Insert(new_insert))
            },
            | Statement::Update(update) => {
                let mut new_update = update.clone();
                new_update.assignments = update
                    .assignments
                    .iter()
                    .map(|a| {
                        Ok(crate::ast::Assignment {
                            column: a.column.clone(),
                            value: substitute_expression(&a.value, params)?,
                        })
                    })
                    .collect::<QSQLResult<Vec<_>>>()?;
                if let Some(where_clause) = &update.where_clause {
                    new_update.where_clause = Some(substitute_expression(where_clause, params)?);
                }
                Ok(Statement::Update(new_update))
            },
            | Statement::Delete(delete) => {
                let mut new_delete = delete.clone();
                if let Some(where_clause) = &delete.where_clause {
                    new_delete.where_clause = Some(substitute_expression(where_clause, params)?);
                }
                Ok(Statement::Delete(new_delete))
            },
            // Other statement types are returned as-is
            | other => Ok(other.clone()),
        }
    }

    substitute_statement(statement, params)
}

// Tests have been moved to tests/prepared_statements_tests.rs
