//! Query Plan Cache with LRU Eviction and Memory Management
//!
//! This module provides a brain-inspired query plan cache with:
//! - Configurable memory limits
//! - LRU (Least Recently Used) eviction based on `last_accessed`
//! - Synaptic strength-based prioritization (Hebbian-inspired)
//! - Automatic eviction under memory pressure

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::query_plan::QueryPlan;

/// Cached query plan with execution statistics and synaptic metadata
#[derive(Debug, Clone, Serialize)]
pub struct CachedQueryPlan {
    /// The cached query plan
    pub plan: Arc<QueryPlan>,
    /// Number of times this plan has been executed
    pub execution_count: u64,
    /// Average execution duration
    pub average_duration: Duration,
    /// Synaptic strength (0.0-1.0) - higher means more important to keep
    /// Inspired by Hebbian learning: "neurons that fire together, wire together"
    pub synaptic_strength: f32,
    /// Last time this plan was accessed
    #[serde(skip)]
    pub last_accessed: Instant,
    /// Estimated memory size in bytes
    pub estimated_size_bytes: usize,
}

impl<'de> Deserialize<'de> for CachedQueryPlan {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct CachedQueryPlanHelper {
            plan: QueryPlan,
            execution_count: u64,
            average_duration: Duration,
            synaptic_strength: f32,
            estimated_size_bytes: usize,
        }

        let helper = CachedQueryPlanHelper::deserialize(deserializer)?;
        Ok(Self {
            plan: Arc::new(helper.plan),
            execution_count: helper.execution_count,
            average_duration: helper.average_duration,
            synaptic_strength: helper.synaptic_strength,
            last_accessed: Instant::now(),
            estimated_size_bytes: helper.estimated_size_bytes,
        })
    }
}

impl CachedQueryPlan {
    /// Create a new cached query plan
    #[must_use]
    pub fn new(plan: Arc<QueryPlan>, duration: Duration) -> Self {
        let estimated_size = Self::estimate_plan_size(&plan);
        Self {
            plan,
            execution_count: 1,
            average_duration: duration,
            synaptic_strength: 0.5, // Initial synaptic strength
            last_accessed: Instant::now(),
            estimated_size_bytes: estimated_size,
        }
    }

    /// Estimate the memory size of a query plan in bytes
    const fn estimate_plan_size(plan: &QueryPlan) -> usize {
        // Base size of the struct
        let base_size = std::mem::size_of::<QueryPlan>();

        // Estimate additional heap allocations based on plan complexity
        let pathway_size = plan.synaptic_pathways.len() * 64; // Approximate per-pathway
        let optimization_size = plan.quantum_optimizations.len() * 32;

        base_size + pathway_size + optimization_size + 256 // Add buffer for string content
    }

    /// Update the plan with a new execution
    pub fn record_execution(&mut self, duration: Duration) {
        self.execution_count += 1;
        self.last_accessed = Instant::now();

        // Update average duration using running average
        let count = self.execution_count;
        self.average_duration = Duration::from_nanos(
            (self.average_duration.as_nanos() as u64 * (count - 1) + duration.as_nanos() as u64)
                / count,
        );

        // Strengthen synaptic connection (Hebbian learning)
        // Frequently used plans become stronger, but cap at 1.0
        self.synaptic_strength = (self.synaptic_strength * 1.05).min(1.0);
    }

    /// Calculate eviction priority (lower = more likely to be evicted)
    /// Combines recency and synaptic strength for Hebbian-inspired eviction
    #[must_use]
    pub fn eviction_priority(&self) -> f64 {
        let recency_factor = self.last_accessed.elapsed().as_secs_f64();
        let strength_factor = f64::from(self.synaptic_strength);
        let usage_factor = (self.execution_count as f64).ln_1p();

        // Lower priority = more likely to evict
        // High recency (old) reduces priority
        // High synaptic strength increases priority
        // High usage increases priority
        strength_factor.mul_add(0.4, usage_factor * 0.3) / recency_factor.mul_add(0.01, 1.0)
    }
}

/// Configuration for the Query Plan Cache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPlanCacheConfig {
    /// Maximum number of cached plans
    pub max_entries: usize,
    /// Maximum memory usage in bytes (0 = unlimited)
    pub max_memory_bytes: usize,
    /// Minimum synaptic strength to keep during eviction (0.0-1.0)
    pub min_synaptic_threshold: f32,
    /// Enable synaptic strength decay over time
    pub enable_synaptic_decay: bool,
    /// Synaptic decay rate per eviction cycle (0.0-1.0)
    pub synaptic_decay_rate: f32,
    /// Number of entries to evict when limit is reached
    pub eviction_batch_size: usize,
}

impl Default for QueryPlanCacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 1000,
            max_memory_bytes: 64 * 1024 * 1024, // 64 MB default
            min_synaptic_threshold: 0.1,
            enable_synaptic_decay: true,
            synaptic_decay_rate: 0.95,
            eviction_batch_size: 10,
        }
    }
}

impl QueryPlanCacheConfig {
    /// Create a config for testing with smaller limits
    #[cfg(test)]
    #[must_use]
    pub const fn testing() -> Self {
        Self {
            max_entries: 10,
            max_memory_bytes: 1024 * 1024, // 1 MB
            min_synaptic_threshold: 0.1,
            enable_synaptic_decay: true,
            synaptic_decay_rate: 0.9,
            eviction_batch_size: 2,
        }
    }
}

/// Query Plan Cache with LRU eviction and memory management
#[derive(Debug)]
pub struct QueryPlanCache {
    /// Cached query plans indexed by query string
    entries: HashMap<String, CachedQueryPlan>,
    /// Cache configuration
    config: QueryPlanCacheConfig,
    /// Current total memory usage (estimated)
    current_memory_bytes: usize,
    /// Statistics
    stats: CacheStatistics,
}

/// Cache statistics for monitoring and debugging
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStatistics {
    /// Total number of cache hits
    pub hits: u64,
    /// Total number of cache misses
    pub misses: u64,
    /// Total number of evictions
    pub evictions: u64,
    /// Total number of insertions
    pub insertions: u64,
    /// Number of eviction cycles triggered
    pub eviction_cycles: u64,
    /// Peak memory usage
    pub peak_memory_bytes: usize,
    /// Peak entry count
    pub peak_entries: usize,
}

impl CacheStatistics {
    /// Calculate cache hit ratio
    #[must_use]
    pub fn hit_ratio(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

impl QueryPlanCache {
    /// Create a new cache with default configuration
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(QueryPlanCacheConfig::default())
    }

    /// Create a new cache with custom configuration
    #[must_use]
    pub fn with_config(config: QueryPlanCacheConfig) -> Self {
        let initial_capacity = config.max_entries.min(100);
        Self {
            entries: HashMap::with_capacity(initial_capacity),
            config,
            current_memory_bytes: 0,
            stats: CacheStatistics::default(),
        }
    }

    /// Get a cached plan by query string
    pub fn get(&mut self, query: &str) -> Option<&CachedQueryPlan> {
        if self.entries.contains_key(query) {
            self.stats.hits += 1;
            // Update last_accessed through get_mut
            if let Some(entry) = self.entries.get_mut(query) {
                entry.last_accessed = Instant::now();
            }
            self.entries.get(query)
        } else {
            self.stats.misses += 1;
            None
        }
    }

    /// Get a mutable reference to a cached plan
    pub fn get_mut(&mut self, query: &str) -> Option<&mut CachedQueryPlan> {
        if let Some(entry) = self.entries.get_mut(query) {
            self.stats.hits += 1;
            entry.last_accessed = Instant::now();
            Some(entry)
        } else {
            self.stats.misses += 1;
            None
        }
    }

    /// Insert a new cached plan, triggering eviction if needed
    pub fn insert(&mut self, query: String, plan: CachedQueryPlan) {
        let entry_size = plan.estimated_size_bytes;

        // Check if we need to evict before inserting
        self.evict_if_needed(entry_size);

        // Remove old entry if exists (to update memory tracking)
        if let Some(old) = self.entries.remove(&query) {
            self.current_memory_bytes = self
                .current_memory_bytes
                .saturating_sub(old.estimated_size_bytes);
        }

        // Insert new entry
        self.current_memory_bytes += entry_size;
        self.entries.insert(query, plan);
        self.stats.insertions += 1;

        // Update peak statistics
        self.stats.peak_memory_bytes = self.stats.peak_memory_bytes.max(self.current_memory_bytes);
        self.stats.peak_entries = self.stats.peak_entries.max(self.entries.len());

        debug!(
            "Cache insert: entries={}, memory={}KB",
            self.entries.len(),
            self.current_memory_bytes / 1024
        );
    }

    /// Check if eviction is needed and perform it
    fn evict_if_needed(&mut self, incoming_size: usize) {
        let needs_entry_eviction = self.entries.len() >= self.config.max_entries;
        let needs_memory_eviction = self.config.max_memory_bytes > 0
            && self.current_memory_bytes + incoming_size > self.config.max_memory_bytes;

        if needs_entry_eviction || needs_memory_eviction {
            self.perform_eviction();
        }
    }

    /// Perform eviction based on LRU and synaptic strength
    fn perform_eviction(&mut self) {
        self.stats.eviction_cycles += 1;

        // Apply synaptic decay if enabled
        if self.config.enable_synaptic_decay {
            for entry in self.entries.values_mut() {
                entry.synaptic_strength *= self.config.synaptic_decay_rate;
            }
        }

        // Collect entries with their eviction priority
        let mut priorities: Vec<(String, f64)> = self
            .entries
            .iter()
            .map(|(k, v)| (k.clone(), v.eviction_priority()))
            .collect();

        // Sort by priority (ascending - lowest priority first to evict)
        priorities.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        // Evict lowest priority entries
        let evict_count = self.config.eviction_batch_size.min(priorities.len());
        let mut evicted_memory = 0usize;
        let mut evicted_count = 0usize;

        for (query, priority) in priorities.into_iter().take(evict_count) {
            // Don't evict entries with high synaptic strength unless we're desperate
            if let Some(entry) = self.entries.get(&query) {
                if entry.synaptic_strength > self.config.min_synaptic_threshold
                    && evicted_count > 0
                    && self.entries.len() > 1
                {
                    debug!(
                        "Skipping eviction of high-strength entry (strength={}): {}",
                        entry.synaptic_strength,
                        &query[..query.len().min(50)]
                    );
                    continue;
                }
            }

            if let Some(entry) = self.entries.remove(&query) {
                evicted_memory += entry.estimated_size_bytes;
                evicted_count += 1;
                self.stats.evictions += 1;

                debug!(
                    "Evicted cache entry (priority={:.4}, strength={:.3}): {}",
                    priority,
                    entry.synaptic_strength,
                    &query[..query.len().min(50)]
                );
            }
        }

        self.current_memory_bytes = self.current_memory_bytes.saturating_sub(evicted_memory);

        info!(
            "Cache eviction complete: evicted={}, freed={}KB, remaining={}",
            evicted_count,
            evicted_memory / 1024,
            self.entries.len()
        );
    }

    /// Manually trigger eviction to reduce memory usage to target
    pub fn evict_to_target_memory(&mut self, target_bytes: usize) {
        while self.current_memory_bytes > target_bytes && !self.entries.is_empty() {
            self.perform_eviction();
        }
    }

    /// Clear all cached entries
    pub fn clear(&mut self) {
        let evicted_count = self.entries.len();
        self.entries.clear();
        self.current_memory_bytes = 0;
        self.stats.evictions += evicted_count as u64;
        info!("Cache cleared: evicted {} entries", evicted_count);
    }

    /// Get cache statistics
    #[must_use]
    pub const fn statistics(&self) -> &CacheStatistics {
        &self.stats
    }

    /// Get current memory usage in bytes
    #[must_use]
    pub const fn current_memory_bytes(&self) -> usize {
        self.current_memory_bytes
    }

    /// Get current number of entries
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if cache is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get cache configuration
    #[must_use]
    pub const fn config(&self) -> &QueryPlanCacheConfig {
        &self.config
    }

    /// Update cache configuration
    /// Note: This may trigger eviction if new limits are lower
    pub fn set_config(&mut self, config: QueryPlanCacheConfig) {
        self.config = config;
        // Trigger eviction if we're over the new limits
        if self.entries.len() > self.config.max_entries
            || (self.config.max_memory_bytes > 0
                && self.current_memory_bytes > self.config.max_memory_bytes)
        {
            self.perform_eviction();
        }
    }

    /// Iterate over all cached entries
    pub fn iter(&self) -> impl Iterator<Item = (&String, &CachedQueryPlan)> {
        self.entries.iter()
    }

    /// Check if a query is cached
    #[must_use]
    pub fn contains(&self, query: &str) -> bool {
        self.entries.contains_key(query)
    }

    /// Get all cached queries (for debugging/testing)
    #[must_use]
    pub fn cached_queries(&self) -> Vec<&String> {
        self.entries.keys().collect()
    }
}

impl Default for QueryPlanCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{SelectStatement, Statement};
    use crate::query_plan::{ExecutionStrategy, OptimizationMetadata};

    fn create_test_plan() -> Arc<QueryPlan> {
        Arc::new(QueryPlan {
            statement: Arc::new(Statement::Select(SelectStatement {
                select_list: vec![],
                from: None,
                where_clause: None,
                group_by: vec![],
                having: None,
                order_by: vec![],
                limit: None,
                offset: None,
                synaptic_weight: None,
                plasticity_threshold: None,
                neuromatch_clause: None,
                quantum_parallel: false,
                grover_iterations: None,
                with_clause: None,
                union_clause: None,
            })),
            execution_strategy: ExecutionStrategy::Sequential,
            synaptic_pathways: vec![],
            quantum_optimizations: vec![],
            estimated_cost: 100.0,
            optimization_metadata: OptimizationMetadata {
                optimization_time: Duration::from_millis(1),
                iterations_used: 1,
                convergence_achieved: true,
                synaptic_adaptations: 0,
                quantum_optimizations_applied: 0,
            },
        })
    }

    #[test]
    fn test_cache_basic_operations() {
        let mut cache = QueryPlanCache::new();

        let plan = create_test_plan();
        let cached = CachedQueryPlan::new(plan, Duration::from_millis(10));

        cache.insert("SELECT * FROM users".to_string(), cached);

        assert_eq!(cache.len(), 1);
        assert!(cache.contains("SELECT * FROM users"));
        assert!(cache.get("SELECT * FROM users").is_some());
        assert!(cache.get("SELECT * FROM other").is_none());

        assert_eq!(cache.statistics().insertions, 1);
        assert_eq!(cache.statistics().hits, 1);
        assert_eq!(cache.statistics().misses, 1);
    }

    #[test]
    fn test_cache_eviction_by_entries() {
        let config = QueryPlanCacheConfig {
            max_entries: 3,
            max_memory_bytes: 0, // Unlimited memory
            eviction_batch_size: 1,
            ..Default::default()
        };
        let mut cache = QueryPlanCache::with_config(config);

        // Insert 4 entries (should trigger eviction on 4th)
        for i in 0..4 {
            let plan = create_test_plan();
            let cached = CachedQueryPlan::new(plan, Duration::from_millis(10));
            cache.insert(format!("SELECT {i} FROM test"), cached);
            // Small delay to ensure different last_accessed times
            std::thread::sleep(Duration::from_millis(1));
        }

        // Should have evicted at least one entry
        assert!(cache.len() <= 3);
        assert!(cache.statistics().evictions > 0);
    }

    #[test]
    fn test_cache_eviction_by_memory() {
        let config = QueryPlanCacheConfig {
            max_entries: 1000,
            max_memory_bytes: 1000, // Very small limit
            eviction_batch_size: 1,
            ..Default::default()
        };
        let mut cache = QueryPlanCache::with_config(config);

        // Insert entries until memory limit triggers eviction
        for i in 0..10 {
            let plan = create_test_plan();
            let cached = CachedQueryPlan::new(plan, Duration::from_millis(10));
            cache.insert(format!("SELECT {i} FROM test"), cached);
        }

        // Should have evicted some entries due to memory pressure
        assert!(cache.current_memory_bytes() <= 1500); // Some tolerance
        assert!(cache.statistics().evictions > 0);
    }

    #[test]
    fn test_synaptic_strength_affects_eviction() {
        let config = QueryPlanCacheConfig {
            max_entries: 2,
            max_memory_bytes: 0,
            min_synaptic_threshold: 0.8,
            eviction_batch_size: 1,
            enable_synaptic_decay: false,
            ..Default::default()
        };
        let mut cache = QueryPlanCache::with_config(config);

        // Insert entry with high synaptic strength
        let plan = create_test_plan();
        let mut cached_strong = CachedQueryPlan::new(plan, Duration::from_millis(10));
        cached_strong.synaptic_strength = 0.9;
        cache.insert("STRONG_QUERY".to_string(), cached_strong);

        // Insert entry with low synaptic strength
        let plan = create_test_plan();
        let mut cached_weak = CachedQueryPlan::new(plan, Duration::from_millis(10));
        cached_weak.synaptic_strength = 0.1;
        cache.insert("WEAK_QUERY".to_string(), cached_weak);

        // Insert third entry to trigger eviction
        let plan = create_test_plan();
        let cached_new = CachedQueryPlan::new(plan, Duration::from_millis(10));
        cache.insert("NEW_QUERY".to_string(), cached_new);

        // Weak query should be evicted first (if eviction happened)
        if cache.statistics().evictions > 0 {
            // Strong query should still be in cache
            assert!(cache.contains("STRONG_QUERY"));
        }
    }

    #[test]
    fn test_cache_statistics() {
        let mut cache = QueryPlanCache::new();

        let plan = create_test_plan();
        let cached = CachedQueryPlan::new(plan, Duration::from_millis(10));
        cache.insert("query1".to_string(), cached);

        // Hit
        cache.get("query1");
        // Miss
        cache.get("query2");

        let stats = cache.statistics();
        assert_eq!(stats.insertions, 1);
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_ratio() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_execution_recording() {
        let plan = create_test_plan();
        let mut cached = CachedQueryPlan::new(plan, Duration::from_millis(100));

        assert_eq!(cached.execution_count, 1);
        let initial_strength = cached.synaptic_strength;

        cached.record_execution(Duration::from_millis(50));

        assert_eq!(cached.execution_count, 2);
        assert!(cached.synaptic_strength > initial_strength);
        assert!(cached.average_duration < Duration::from_millis(100));
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = QueryPlanCache::new();

        for i in 0..5 {
            let plan = create_test_plan();
            let cached = CachedQueryPlan::new(plan, Duration::from_millis(10));
            cache.insert(format!("query{i}"), cached);
        }

        assert_eq!(cache.len(), 5);

        cache.clear();

        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
        assert_eq!(cache.current_memory_bytes(), 0);
    }

    #[test]
    fn test_eviction_priority_calculation() {
        let plan = create_test_plan();
        let mut cached = CachedQueryPlan::new(plan, Duration::from_millis(10));

        let initial_priority = cached.eviction_priority();

        // Strengthen the connection
        cached.synaptic_strength = 0.9;
        cached.execution_count = 100;

        let strengthened_priority = cached.eviction_priority();

        // Higher strength and usage should mean higher priority (less likely to evict)
        assert!(strengthened_priority > initial_priority);
    }
}
