//! Natural Language Processing for QSQL
//!
//! This module provides natural language understanding and translation
//! capabilities for converting human language queries into QSQL syntax.

use crate::ast::*;
use crate::error::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, instrument};
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

/// Natural Language Processor for QSQL translation
#[derive(Debug, Clone)]
pub struct NaturalLanguageProcessor {
    intent_patterns: HashMap<QueryIntent, Vec<Regex>>,
    entity_extractors: HashMap<EntityType, Regex>,
    synonym_map: HashMap<String, String>,
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
            synonym_map: HashMap::new(),
            table_mappings: HashMap::new(),
            column_mappings: HashMap::new(),
        };

        processor.initialize_patterns()?;
        processor.initialize_synonyms();
        processor.initialize_mappings();
        processor.initialize_entity_extractors()?;

        Ok(processor)
    }

    /// Translate natural language to QSQL
    #[instrument(skip(self))]
    pub fn translate_to_qsql(&self, natural_query: &str) -> QSQLResult<String> {
        debug!("Translating natural language query: {}", natural_query);

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
        text.to_lowercase()
            .trim()
            .to_string()
    }

    /// Classify query intent using pattern matching
    fn classify_intent(&self, query: &str) -> QSQLResult<QueryIntent> {
        let mut best_match = (QueryIntent::Select, 0.0f32);

        for (intent, patterns) in &self.intent_patterns {
            for pattern in patterns {
                if let Some(matches) = pattern.find(query) {
                    let confidence = matches.len() as f32 / query.len() as f32;
                    if confidence > best_match.1 {
                        best_match = (intent.clone(), confidence);
                    }
                }
            }
        }

        if best_match.1 > 0.1 {
            Ok(best_match.0)
        } else {
            Err(NLPError::IntentRecognitionFailed {
                text: query.to_string(),
            }.into())
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

        let entity_confidence: f32 = entities.iter().map(|e| e.confidence).sum::<f32>() / entities.len() as f32;
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
    fn generate_qsql_from_components(&self, intent: &QueryIntent, entities: &[Entity]) -> QSQLResult<String> {
        match intent {
            QueryIntent::Select => self.generate_select_query(entities),
            QueryIntent::NeuroMatch => self.generate_neuromatch_query(entities),
            QueryIntent::QuantumSearch => self.generate_quantum_search_query(entities),
            QueryIntent::Filter => self.generate_filter_query(entities),
            QueryIntent::Aggregate => self.generate_aggregate_query(entities),
            QueryIntent::Join => self.generate_join_query(entities),
            _ => Err(NLPError::UnsupportedConstruct {
                construct: format!("{:?}", intent),
            }.into()),
        }
    }

    /// Generate SELECT query from entities
    fn generate_select_query(&self, entities: &[Entity]) -> QSQLResult<String> {
        let mut query = String::from("SELECT ");

        // Extract columns
        let columns: Vec<&Entity> = entities.iter()
            .filter(|e| e.entity_type == EntityType::ColumnName)
            .collect();

        if columns.is_empty() {
            query.push_str("*");
        } else {
            let column_names: Vec<String> = columns.iter()
                .map(|e| self.map_column_name(&e.value))
                .collect();
            query.push_str(&column_names.join(", "));
        }

        // Extract table
        let tables: Vec<&Entity> = entities.iter()
            .filter(|e| e.entity_type == EntityType::TableName)
            .collect();

        if let Some(table) = tables.first() {
            query.push_str(" FROM ");
            query.push_str(&self.map_table_name(&table.value));
        } else {
            return Err(NLPError::EntityExtractionFailed {
                text: "No table name found".to_string(),
            }.into());
        }

        // Add WHERE conditions
        let conditions = self.extract_conditions(entities)?;
        if !conditions.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&conditions.join(" AND "));
        }

        Ok(query)
    }

    /// Generate NEUROMATCH query
    fn generate_neuromatch_query(&self, entities: &[Entity]) -> QSQLResult<String> {
        let mut query = String::from("NEUROMATCH ");

        // Extract table
        let tables: Vec<&Entity> = entities.iter()
            .filter(|e| e.entity_type == EntityType::TableName)
            .collect();

        if let Some(table) = tables.first() {
            query.push_str(&self.map_table_name(&table.value));
        } else {
            return Err(NLPError::EntityExtractionFailed {
                text: "No table name found for NEUROMATCH".to_string(),
            }.into());
        }

        // Add pattern conditions
        let conditions = self.extract_conditions(entities)?;
        if !conditions.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&conditions.join(" AND "));
        }

        // Extract synaptic weight
        let weights: Vec<&Entity> = entities.iter()
            .filter(|e| e.entity_type == EntityType::NeuromorphicWeight)
            .collect();

        let weight = if let Some(w) = weights.first() {
            w.value.clone()
        } else {
            "0.8".to_string() // Default weight
        };

        query.push_str(&format!(" WITH SYNAPTIC_WEIGHT {}", weight));

        Ok(query)
    }

    /// Generate QUANTUM_SEARCH query
    fn generate_quantum_search_query(&self, entities: &[Entity]) -> QSQLResult<String> {
        let mut query = String::from("QUANTUM_SEARCH ");

        // Extract table
        let tables: Vec<&Entity> = entities.iter()
            .filter(|e| e.entity_type == EntityType::TableName)
            .collect();

        if let Some(table) = tables.first() {
            query.push_str(&self.map_table_name(&table.value));
        } else {
            return Err(NLPError::EntityExtractionFailed {
                text: "No table name found for QUANTUM_SEARCH".to_string(),
            }.into());
        }

        // Add search conditions
        let conditions = self.extract_conditions(entities)?;
        if !conditions.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&conditions.join(" AND "));
        }

        // Add quantum parameters
        query.push_str(" WITH AMPLITUDE_AMPLIFICATION");

        Ok(query)
    }

    /// Generate filter query
    fn generate_filter_query(&self, entities: &[Entity]) -> QSQLResult<String> {
        self.generate_select_query(entities) // Filter is essentially a SELECT with WHERE
    }

    /// Generate aggregate query
    fn generate_aggregate_query(&self, entities: &[Entity]) -> QSQLResult<String> {
        let mut query = String::from("SELECT ");

        // Extract aggregation functions
        let aggregations: Vec<&Entity> = entities.iter()
            .filter(|e| e.entity_type == EntityType::Aggregation)
            .collect();

        if !aggregations.is_empty() {
            let agg_exprs: Vec<String> = aggregations.iter()
                .map(|e| self.map_aggregation(&e.value))
                .collect();
            query.push_str(&agg_exprs.join(", "));
        } else {
            query.push_str("COUNT(*)");
        }

        // Add FROM clause
        let tables: Vec<&Entity> = entities.iter()
            .filter(|e| e.entity_type == EntityType::TableName)
            .collect();

        if let Some(table) = tables.first() {
            query.push_str(" FROM ");
            query.push_str(&self.map_table_name(&table.value));
        }

        Ok(query)
    }

    /// Generate join query
    fn generate_join_query(&self, entities: &[Entity]) -> QSQLResult<String> {
        // Simplified join generation
        let mut query = String::from("SELECT * FROM ");

        let tables: Vec<&Entity> = entities.iter()
            .filter(|e| e.entity_type == EntityType::TableName)
            .collect();

        if tables.len() >= 2 {
            query.push_str(&self.map_table_name(&tables[0].value));
            query.push_str(" JOIN ");
            query.push_str(&self.map_table_name(&tables[1].value));
            query.push_str(" ON "); // Add default join condition
            query.push_str(&format!("{}.id = {}.id",
                self.map_table_name(&tables[0].value),
                self.map_table_name(&tables[1].value)
            ));
        }

        Ok(query)
    }

    /// Extract WHERE conditions from entities
    fn extract_conditions(&self, entities: &[Entity]) -> QSQLResult<Vec<String>> {
        let mut conditions = Vec::new();

        let mut i = 0;
        while i < entities.len() {
            if entities[i].entity_type == EntityType::ColumnName {
                if i + 2 < entities.len()
                    && entities[i + 1].entity_type == EntityType::Operator
                    && entities[i + 2].entity_type == EntityType::Value {

                    let column = self.map_column_name(&entities[i].value);
                    let operator = self.map_operator(&entities[i + 1].value);
                    let value = self.format_value(&entities[i + 2].value);

                    conditions.push(format!("{} {} {}", column, operator, value));
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

    /// Map natural language table names to actual table names
    fn map_table_name(&self, name: &str) -> String {
        self.table_mappings.get(name)
            .cloned()
            .unwrap_or_else(|| name.to_string())
    }

    /// Map natural language column names to actual column names
    fn map_column_name(&self, name: &str) -> String {
        self.column_mappings.get(name)
            .cloned()
            .unwrap_or_else(|| name.to_string())
    }

    /// Map natural language operators to SQL operators
    fn map_operator(&self, op: &str) -> String {
        match op.to_lowercase().as_str() {
            "is" | "equals" | "equal to" => "=".to_string(),
            "greater than" | "more than" | "above" => ">".to_string(),
            "less than" | "below" | "under" => "<".to_string(),
            "contains" | "includes" => "LIKE".to_string(),
            _ => "=".to_string(),
        }
    }

    /// Map aggregation functions
    fn map_aggregation(&self, agg: &str) -> String {
        match agg.to_lowercase().as_str() {
            "count" | "number of" => "COUNT(*)".to_string(),
            "sum" | "total" => "SUM(amount)".to_string(),
            "average" | "avg" | "mean" => "AVG(amount)".to_string(),
            "maximum" | "max" | "highest" => "MAX(amount)".to_string(),
            "minimum" | "min" | "lowest" => "MIN(amount)".to_string(),
            _ => "COUNT(*)".to_string(),
        }
    }

    /// Format values for SQL
    fn format_value(&self, value: &str) -> String {
        // Try to parse as number
        if value.parse::<i64>().is_ok() || value.parse::<f64>().is_ok() {
            value.to_string()
        } else {
            format!("'{}'", value.replace("'", "''")) // Escape single quotes
        }
    }

    /// Initialize intent patterns
    fn initialize_patterns(&mut self) -> QSQLResult<()> {
        // Select patterns
        self.intent_patterns.insert(QueryIntent::Select, vec![
            Regex::new(r"(?i)\b(show|display|list|get|find|select)\b").unwrap(),
            Regex::new(r"(?i)\b(what|which|who)\b").unwrap(),
        ]);

        // NeuroMatch patterns
        self.intent_patterns.insert(QueryIntent::NeuroMatch, vec![
            Regex::new(r"(?i)\b(neural|neuromorphic|synaptic|brain|pattern)\b").unwrap(),
            Regex::new(r"(?i)\b(match|similar|like|resembles)\b").unwrap(),
        ]);

        // Quantum patterns
        self.intent_patterns.insert(QueryIntent::QuantumSearch, vec![
            Regex::new(r"(?i)\b(quantum|superposition|entangled|grover)\b").unwrap(),
            Regex::new(r"(?i)\b(search|find|locate)\b").unwrap(),
        ]);

        // Filter patterns
        self.intent_patterns.insert(QueryIntent::Filter, vec![
            Regex::new(r"(?i)\b(where|filter|condition|criteria)\b").unwrap(),
        ]);

        // Aggregate patterns
        self.intent_patterns.insert(QueryIntent::Aggregate, vec![
            Regex::new(r"(?i)\b(count|sum|total|average|max|min|group)\b").unwrap(),
        ]);

        // Join patterns
        self.intent_patterns.insert(QueryIntent::Join, vec![
            Regex::new(r"(?i)\b(join|combine|merge|connect)\b").unwrap(),
        ]);

        Ok(())
    }

    /// Initialize entity extractors
    fn initialize_entity_extractors(&mut self) -> QSQLResult<()> {
        // Table name patterns
        self.entity_extractors.insert(EntityType::TableName,
            Regex::new(r"(?i)\b(users|products|orders|customers|items|data)\b").unwrap());

        // Column name patterns
        self.entity_extractors.insert(EntityType::ColumnName,
            Regex::new(r"(?i)\b(name|age|price|id|email|status|date|amount)\b").unwrap());

        // Number patterns
        self.entity_extractors.insert(EntityType::Number,
            Regex::new(r"\b\d+(?:\.\d+)?\b").unwrap());

        // Operator patterns
        self.entity_extractors.insert(EntityType::Operator,
            Regex::new(r"(?i)\b(equals?|greater than|less than|above|below|contains|like)\b").unwrap());

        // Neuromorphic weight patterns
        self.entity_extractors.insert(EntityType::NeuromorphicWeight,
            Regex::new(r"(?i)weight\s+(\d+(?:\.\d+)?)").unwrap());

        Ok(())
    }

    /// Initialize synonyms
    fn initialize_synonyms(&mut self) {
        self.synonym_map.insert("people".to_string(), "users".to_string());
        self.synonym_map.insert("customers".to_string(), "users".to_string());
        self.synonym_map.insert("items".to_string(), "products".to_string());
        self.synonym_map.insert("goods".to_string(), "products".to_string());
        self.synonym_map.insert("purchases".to_string(), "orders".to_string());
        self.synonym_map.insert("transactions".to_string(), "orders".to_string());
    }

    /// Initialize table and column mappings
    fn initialize_mappings(&mut self) {
        // Table mappings
        self.table_mappings.insert("people".to_string(), "users".to_string());
        self.table_mappings.insert("customers".to_string(), "users".to_string());
        self.table_mappings.insert("items".to_string(), "products".to_string());

        // Column mappings
        self.column_mappings.insert("full name".to_string(), "name".to_string());
        self.column_mappings.insert("cost".to_string(), "price".to_string());
        self.column_mappings.insert("value".to_string(), "amount".to_string());
    }
}

impl Default for NaturalLanguageProcessor {
    fn default() -> Self {
        Self::new().expect("Failed to create NaturalLanguageProcessor")
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
        let entities = nlp.extract_entities("show users where age greater than 25").unwrap();
        assert!(!entities.is_empty());

        let has_table = entities.iter().any(|e| e.entity_type == EntityType::TableName);
        let has_number = entities.iter().any(|e| e.entity_type == EntityType::Number);
        assert!(has_table && has_number);
    }
}
