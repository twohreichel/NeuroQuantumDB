//! Tests for query plan cache functionality
//!
//! Extracted from src/query_plan_cache.rs inline tests

use std::sync::Arc;
use std::time::Duration;

use neuroquantum_qsql::query_plan::{ExecutionStrategy, OptimizationMetadata, QueryPlan};
use neuroquantum_qsql::query_plan_cache::{
    CachedQueryPlan, QueryPlanCache, QueryPlanCacheConfig,
};
use neuroquantum_qsql::ast::{SelectStatement, Statement};

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
