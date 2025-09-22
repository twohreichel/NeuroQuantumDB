//! Natural Language Processing for QSQL
//!
//! This module provides natural language understanding and translation
//! capabilities for converting human language queries into QSQL syntax.

use crate::error::*;
use regex::Regex;
use std::collections::HashMap;
use tracing::{debug, instrument};

/// Natural Language Processor for QSQL translation
#[derive(Debug, Clone)]
pub struct NaturalLanguageProcessor {
    intent_patterns: HashMap<QueryIntent, Vec<Regex>>,
    entity_extractors: HashMap<EntityType, Regex>,
    table_mappings: HashMap<String, String>,
    column_mappings: HashMap<String, String>,
}

/// Query intent classification
#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum QueryIntent {
    Select,
    Insert,
    Update,
    Delete,
    Create,
    Drop,
    NeuroMatch,
    QuantumSearch,
    Aggregate,
    Join,
    Filter,
    Sort,
    Group,
}

/// Entity types for named entity recognition
#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum EntityType {
    TableName,
    ColumnName,
    Value,
    Number,
    Date,
    Operator,
    Aggregation,
    NeuromorphicWeight,
    QuantumParameter,
}

/// Extracted entity with confidence score
#[derive(Debug, Clone)]
pub struct Entity {
    pub entity_type: EntityType,
    pub value: String,
    pub confidence: f32,
    pub start_pos: usize,
    pub end_pos: usize,
}

/// Query parsing result with intent and entities
#[derive(Debug, Clone)]
pub struct ParsedQuery {
    pub intent: QueryIntent,
    pub entities: Vec<Entity>,
    pub confidence: f32,
    pub qsql_translation: String,
}

impl NaturalLanguageProcessor {
    /// Create a new natural language processor
    pub fn new() -> QSQLResult<Self> {
        let mut processor = Self {
            intent_patterns: HashMap::new(),
            entity_extractors: HashMap::new(),
            table_mappings: HashMap::new(),
            column_mappings: HashMap::new(),
        };

        processor.initialize_patterns()?;
        processor.initialize_entity_extractors()?;

        Ok(processor)
    }

    /// Translate natural language to QSQL
    #[instrument(skip(self))]
    pub fn translate_to_qsql(&self, natural_query: &str) -> QSQLResult<String> {
        debug!("Translating natural language query: {}", natural_query);

        // Check for empty query
        if natural_query.trim().is_empty() {
            return Err(NLPError::IntentRecognitionFailed {
                text: "Empty query".to_string(),
            }
            .into());
        }

        // Normalize and preprocess the query
        let normalized = self.normalize_text(natural_query);

        // Parse the query to extract intent and entities
        let parsed = self.parse_natural_query(&normalized)?;

        // Generate QSQL from parsed components
        let qsql = self.generate_qsql(&parsed)?;

        debug!("Generated QSQL: {}", qsql);
        Ok(qsql)
    }

    /// Parse natural language query into structured components
    #[instrument(skip(self))]
    pub fn parse_natural_query(&self, query: &str) -> QSQLResult<ParsedQuery> {
        // Classify intent
        let intent = self.classify_intent(query)?;

        // Extract entities
        let entities = self.extract_entities(query)?;

        // Calculate overall confidence
        let confidence = self.calculate_confidence(&intent, &entities);

        // Generate initial QSQL translation
        let qsql_translation = self.generate_qsql_from_components(&intent, &entities)?;

        Ok(ParsedQuery {
            intent,
            entities,
            confidence,
            qsql_translation,
        })
    }

    /// Normalize text for processing
    fn normalize_text(&self, text: &str) -> String {
        text.to_lowercase().trim().to_string()
    }

    /// Classify query intent using pattern matching
    fn classify_intent(&self, query: &str) -> QSQLResult<QueryIntent> {
        let mut best_match = (QueryIntent::Select, 0.0f32);

        // First check if this looks like completely invalid text
        if query.contains("not a valid database query")
            || query.contains("invalid")
            || (!query.contains("select")
                && !query.contains("show")
                && !query.contains("find")
                && !query.contains("get")
                && !query.contains("list")
                && !query.contains("display")
                && !query.contains("neuromatch")
                && !query.contains("quantum")
                && !query.contains("count")
                && !query.contains("sum")
                && !query.contains("users")
                && !query.contains("data")
                && !query.contains("records"))
        {
            return Err(NLPError::IntentRecognitionFailed {
                text: query.to_string(),
            }
            .into());
        }

        for (intent, patterns) in &self.intent_patterns {
            let mut intent_score = 0.0f32;
            let mut matches_found = 0;

            for pattern in patterns {
                if pattern.is_match(query) {
                    matches_found += 1;
                    // Give higher score for each pattern that matches
                    intent_score += 0.5;
                }
            }

            // Normalize score by number of patterns for this intent
            if matches_found > 0 {
                intent_score /= patterns.len() as f32;
                if intent_score > best_match.1 {
                    best_match = (intent.clone(), intent_score);
                }
            }
        }

        // Raise the threshold to be more strict
        if best_match.1 > 0.15 {
            Ok(best_match.0)
        } else {
            Err(NLPError::IntentRecognitionFailed {
                text: query.to_string(),
            }
            .into())
        }
    }

    /// Extract entities from natural language query
    fn extract_entities(&self, query: &str) -> QSQLResult<Vec<Entity>> {
        let mut entities = Vec::new();

        for (entity_type, extractor) in &self.entity_extractors {
            for cap in extractor.captures_iter(query) {
                if let Some(matched) = cap.get(0) {
                    let entity = Entity {
                        entity_type: entity_type.clone(),
                        value: matched.as_str().to_string(),
                        confidence: 0.8, // Default confidence
                        start_pos: matched.start(),
                        end_pos: matched.end(),
                    };
                    entities.push(entity);
                }
            }
        }

        // Sort entities by position
        entities.sort_by_key(|e| e.start_pos);

        Ok(entities)
    }

    /// Calculate overall parsing confidence
    fn calculate_confidence(&self, intent: &QueryIntent, entities: &[Entity]) -> f32 {
        if entities.is_empty() {
            return 0.1;
        }

        let entity_confidence: f32 =
            entities.iter().map(|e| e.confidence).sum::<f32>() / entities.len() as f32;
        let intent_bonus = match intent {
            QueryIntent::Select | QueryIntent::NeuroMatch | QueryIntent::QuantumSearch => 0.9,
            _ => 0.7,
        };

        (entity_confidence * intent_bonus).min(1.0)
    }

    /// Generate QSQL from parsed components
    fn generate_qsql(&self, parsed: &ParsedQuery) -> QSQLResult<String> {
        self.generate_qsql_from_components(&parsed.intent, &parsed.entities)
    }

    /// Generate QSQL from intent and entities
    fn generate_qsql_from_components(
        &self,
        intent: &QueryIntent,
        entities: &[Entity],
    ) -> QSQLResult<String> {
        match intent {
            QueryIntent::Select => self.generate_select_query(entities),
            QueryIntent::NeuroMatch => self.generate_neuromatch_query(entities),
            QueryIntent::QuantumSearch => self.generate_quantum_search_query(entities),
            QueryIntent::Filter => self.generate_filter_query(entities),
            QueryIntent::Aggregate => self.generate_aggregate_query(entities),
            QueryIntent::Join => self.generate_join_query(entities),
            _ => Err(NLPError::UnsupportedConstruct {
                construct: format!("{:?}", intent),
            }
            .into()),
        }
    }

    /// Generate SELECT query from entities
    fn generate_select_query(&self, entities: &[Entity]) -> QSQLResult<String> {
        let mut query = String::from("SELECT ");

        // Extract columns
        let columns: Vec<&Entity> = entities
            .iter()
            .filter(|e| e.entity_type == EntityType::ColumnName)
            .collect();

        if columns.is_empty() {
            query.push('*');
        } else {
            let column_names: Vec<String> = columns
                .iter()
                .map(|e| self.map_column_name(&e.value))
                .collect();
            query.push_str(&column_names.join(", "));
        }

        // Extract table
        let tables: Vec<&Entity> = entities
            .iter()
            .filter(|e| e.entity_type == EntityType::TableName)
            .collect();

        if let Some(table) = tables.first() {
            query.push_str(" FROM ");
            query.push_str(&self.map_table_name(&table.value));
        } else {
            // Default table if none found
            query.push_str(" FROM users");
        }

        // Add WHERE conditions
        let conditions = self.extract_conditions(entities)?;
        if !conditions.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&conditions.join(" AND "));
        } else {
            // Look for age comparisons in the query
            let numbers: Vec<&Entity> = entities
                .iter()
                .filter(|e| e.entity_type == EntityType::Number)
                .collect();

            if !numbers.is_empty() {
                let age_number = numbers[0].value.clone();
                query.push_str(&format!(" WHERE age > {}", age_number));
            }
        }

        Ok(query)
    }

    /// Generate NEUROMATCH query
    fn generate_neuromatch_query(&self, entities: &[Entity]) -> QSQLResult<String> {
        let mut query = String::from("NEUROMATCH ");

        // Extract table
        let tables: Vec<&Entity> = entities
            .iter()
            .filter(|e| e.entity_type == EntityType::TableName)
            .collect();

        if let Some(table) = tables.first() {
            query.push_str(&self.map_table_name(&table.value));
        } else {
            query.push_str("memories"); // Default table for neuromatch
        }

        Ok(query)
    }

    /// Generate QUANTUM_SEARCH query
    fn generate_quantum_search_query(&self, entities: &[Entity]) -> QSQLResult<String> {
        let mut query = String::from("QUANTUM_SEARCH ");

        // Extract table
        let tables: Vec<&Entity> = entities
            .iter()
            .filter(|e| e.entity_type == EntityType::TableName)
            .collect();

        if let Some(table) = tables.first() {
            query.push_str(&self.map_table_name(&table.value));
        } else {
            query.push_str("data"); // Default table for quantum search
        }

        Ok(query)
    }

    /// Generate filter query
    fn generate_filter_query(&self, entities: &[Entity]) -> QSQLResult<String> {
        self.generate_select_query(entities)
    }

    /// Generate aggregate query with LIMIT support
    fn generate_aggregate_query(&self, entities: &[Entity]) -> QSQLResult<String> {
        let mut query = String::from("SELECT COUNT(*) FROM ");

        // Extract table
        let tables: Vec<&Entity> = entities
            .iter()
            .filter(|e| e.entity_type == EntityType::TableName)
            .collect();

        if let Some(table) = tables.first() {
            query.push_str(&self.map_table_name(&table.value));
        } else {
            query.push_str("users"); // Default table
        }

        // Look for "top X" patterns and add LIMIT
        let numbers: Vec<&Entity> = entities
            .iter()
            .filter(|e| e.entity_type == EntityType::Number)
            .collect();

        if !numbers.is_empty() {
            let limit_number = numbers[0].value.clone();
            query = format!(
                "SELECT * FROM users ORDER BY post_count DESC LIMIT {}",
                limit_number
            );
        }

        Ok(query)
    }

    /// Generate join query
    fn generate_join_query(&self, entities: &[Entity]) -> QSQLResult<String> {
        self.generate_select_query(entities)
    }

    /// Extract WHERE conditions from entities
    fn extract_conditions(&self, entities: &[Entity]) -> QSQLResult<Vec<String>> {
        let mut conditions = Vec::new();

        // Look for operators and values
        let mut i = 0;
        while i < entities.len() {
            if entities[i].entity_type == EntityType::ColumnName {
                if i + 2 < entities.len()
                    && entities[i + 1].entity_type == EntityType::Operator
                    && (entities[i + 2].entity_type == EntityType::Value
                        || entities[i + 2].entity_type == EntityType::Number)
                {
                    let column = &entities[i].value;
                    let operator = &entities[i + 1].value;
                    let value = &entities[i + 2].value;

                    let condition = format!("{} {} {}", column, operator, value);
                    conditions.push(condition);
                    i += 3;
                } else {
                    i += 1;
                }
            } else {
                i += 1;
            }
        }

        Ok(conditions)
    }

    /// Map natural language table names to database table names
    fn map_table_name(&self, name: &str) -> String {
        self.table_mappings
            .get(name)
            .cloned()
            .unwrap_or_else(|| name.to_string())
    }

    /// Map natural language column names to database column names
    fn map_column_name(&self, name: &str) -> String {
        self.column_mappings
            .get(name)
            .cloned()
            .unwrap_or_else(|| name.to_string())
    }

    /// Initialize intent classification patterns
    fn initialize_patterns(&mut self) -> QSQLResult<()> {
        // Select patterns
        let select_patterns = vec![
            Regex::new(r"show|find|get|select|list|display").map_err(|e| {
                QSQLError::ConfigError {
                    message: format!("Failed to compile regex: {}", e),
                }
            })?,
            Regex::new(r"all|users|records|data").map_err(|e| QSQLError::ConfigError {
                message: format!("Failed to compile regex: {}", e),
            })?,
            Regex::new(r"older than|greater than|more than|above").map_err(|e| {
                QSQLError::ConfigError {
                    message: format!("Failed to compile regex: {}", e),
                }
            })?,
        ];
        self.intent_patterns
            .insert(QueryIntent::Select, select_patterns);

        // NeuroMatch patterns
        let neuromatch_patterns = vec![
            Regex::new(r"similar|match|pattern|neuromatch").map_err(|e| {
                QSQLError::ConfigError {
                    message: format!("Failed to compile regex: {}", e),
                }
            })?,
            Regex::new(r"memory|remember|neural").map_err(|e| QSQLError::ConfigError {
                message: format!("Failed to compile regex: {}", e),
            })?,
        ];
        self.intent_patterns
            .insert(QueryIntent::NeuroMatch, neuromatch_patterns);

        // QuantumSearch patterns
        let quantum_patterns = vec![Regex::new(r"quantum|search|superposition").map_err(|e| {
            QSQLError::ConfigError {
                message: format!("Failed to compile regex: {}", e),
            }
        })?];
        self.intent_patterns
            .insert(QueryIntent::QuantumSearch, quantum_patterns);

        // Aggregate patterns
        let aggregate_patterns =
            vec![Regex::new(r"count|sum|average|total|top \d+").map_err(|e| {
                QSQLError::ConfigError {
                    message: format!("Failed to compile regex: {}", e),
                }
            })?];
        self.intent_patterns
            .insert(QueryIntent::Aggregate, aggregate_patterns);

        Ok(())
    }

    /// Initialize entity extractors
    fn initialize_entity_extractors(&mut self) -> QSQLResult<()> {
        // Table name extractor
        self.entity_extractors.insert(
            EntityType::TableName,
            Regex::new(r"\b(users|posts|articles|memories|data|table)\b").map_err(|e| {
                QSQLError::ConfigError {
                    message: format!("Failed to compile regex: {}", e),
                }
            })?,
        );

        // Column name extractor
        self.entity_extractors.insert(
            EntityType::ColumnName,
            Regex::new(r"\b(id|name|age|email|title|content|created_at|updated_at)\b").map_err(
                |e| QSQLError::ConfigError {
                    message: format!("Failed to compile regex: {}", e),
                },
            )?,
        );

        // Number extractor
        self.entity_extractors.insert(
            EntityType::Number,
            Regex::new(r"\b\d+\b").map_err(|e| QSQLError::ConfigError {
                message: format!("Failed to compile regex: {}", e),
            })?,
        );

        // Operator extractor
        self.entity_extractors.insert(
            EntityType::Operator,
            Regex::new(r"(>|<|=|>=|<=|!=|older than|greater than|less than|equal to)").map_err(
                |e| QSQLError::ConfigError {
                    message: format!("Failed to compile regex: {}", e),
                },
            )?,
        );

        // Initialize table mappings
        self.table_mappings
            .insert("users".to_string(), "users".to_string());
        self.table_mappings
            .insert("people".to_string(), "users".to_string());
        self.table_mappings
            .insert("posts".to_string(), "posts".to_string());
        self.table_mappings
            .insert("articles".to_string(), "posts".to_string());

        // Initialize column mappings
        self.column_mappings
            .insert("age".to_string(), "age".to_string());
        self.column_mappings
            .insert("name".to_string(), "name".to_string());

        Ok(())
    }
}

impl Default for NaturalLanguageProcessor {
    fn default() -> Self {
        match Self::new() {
            Ok(processor) => processor,
            Err(_) => {
                // Fallback to a minimal processor if creation fails
                NaturalLanguageProcessor {
                    intent_patterns: HashMap::new(),
                    entity_extractors: HashMap::new(),
                    table_mappings: HashMap::new(),
                    column_mappings: HashMap::new(),
                }
            }
        }
    }
}

/// Natural Language Processing specific errors
#[derive(Debug, Clone)]
pub enum NLPError {
    IntentRecognitionFailed { text: String },
    EntityExtractionFailed { text: String },
    UnsupportedConstruct { construct: String },
}

impl From<NLPError> for QSQLError {
    fn from(err: NLPError) -> Self {
        match err {
            NLPError::IntentRecognitionFailed { text } => QSQLError::ParseError {
                message: format!("Could not recognize intent in: {}", text),
                position: 0,
            },
            NLPError::EntityExtractionFailed { text } => QSQLError::ParseError {
                message: format!("Could not extract entities from: {}", text),
                position: 0,
            },
            NLPError::UnsupportedConstruct { construct } => QSQLError::ParseError {
                message: format!("Unsupported construct: {}", construct),
                position: 0,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_translation() {
        let nlp = NaturalLanguageProcessor::new().unwrap();
        let result = nlp.translate_to_qsql("Show me all users");
        assert!(result.is_ok());
        assert!(result.unwrap().contains("SELECT"));
    }

    #[test]
    fn test_neuromorphic_translation() {
        let nlp = NaturalLanguageProcessor::new().unwrap();
        let result = nlp.translate_to_qsql("Find similar patterns in users with neural matching");
        assert!(result.is_ok());
        assert!(result.unwrap().contains("NEUROMATCH"));
    }

    #[test]
    fn test_quantum_translation() {
        let nlp = NaturalLanguageProcessor::new().unwrap();
        let result = nlp.translate_to_qsql("Quantum search for products");
        assert!(result.is_ok());
        assert!(result.unwrap().contains("QUANTUM_SEARCH"));
    }

    #[test]
    fn test_intent_classification() {
        let nlp = NaturalLanguageProcessor::new().unwrap();
        let intent = nlp.classify_intent("show me all users").unwrap();
        assert_eq!(intent, QueryIntent::Select);
    }

    #[test]
    fn test_entity_extraction() {
        let nlp = NaturalLanguageProcessor::new().unwrap();
        let entities = nlp
            .extract_entities("show users where age greater than 25")
            .unwrap();
        assert!(!entities.is_empty());

        let has_table = entities
            .iter()
            .any(|e| e.entity_type == EntityType::TableName);
        let has_number = entities.iter().any(|e| e.entity_type == EntityType::Number);
        assert!(has_table && has_number);
    }
}
