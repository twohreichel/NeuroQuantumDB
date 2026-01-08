# QSQL Syntax Examples

This guide provides comprehensive examples of QSQL syntax, from standard SQL operations to advanced neuromorphic and quantum-inspired features.

## Table of Contents

- [Standard SQL Examples](#standard-sql-examples)
- [Neuromorphic Extensions](#neuromorphic-extensions)
- [Performance Tips](#performance-tips)
- [Real-World Use Cases](#real-world-use-cases)

---

## Standard SQL Examples

### Complex JOINs with Multiple Tables

#### Example 1: Three-Table JOIN

```sql
-- Join orders with customers and products to get complete order details
SELECT 
    o.id AS order_id,
    c.name AS customer_name,
    c.email,
    p.name AS product_name,
    p.price,
    o.quantity,
    (p.price * o.quantity) AS total_amount
FROM orders o
INNER JOIN customers c ON o.customer_id = c.id
INNER JOIN products p ON o.product_id = p.id
WHERE o.created_at > '2024-01-01'
ORDER BY o.created_at DESC;
```

#### Example 2: LEFT JOIN with Multiple Conditions

```sql
-- Get all customers and their orders, including customers without orders
SELECT 
    c.id,
    c.name,
    c.email,
    COUNT(o.id) AS order_count,
    COALESCE(SUM(o.total), 0) AS total_spent
FROM customers c
LEFT JOIN orders o ON c.id = o.customer_id 
    AND o.status = 'completed'
    AND o.created_at >= '2024-01-01'
GROUP BY c.id, c.name, c.email
HAVING COUNT(o.id) > 0 OR c.vip_status = true
ORDER BY total_spent DESC;
```

#### Example 3: Self-JOIN for Hierarchical Data

```sql
-- Find employees and their managers
SELECT 
    e.id,
    e.name AS employee_name,
    e.position,
    m.name AS manager_name,
    m.position AS manager_position
FROM employees e
LEFT JOIN employees m ON e.manager_id = m.id
WHERE e.active = true
ORDER BY m.name, e.name;
```

#### Example 4: CROSS JOIN for Combinations

```sql
-- Generate all possible product-category combinations for analysis
SELECT 
    p.name AS product_name,
    c.name AS category_name,
    p.price,
    c.commission_rate,
    (p.price * c.commission_rate) AS commission
FROM products p
CROSS JOIN categories c
WHERE p.active = true AND c.active = true
LIMIT 100;
```

### Subqueries (Correlated and Non-Correlated)

#### Example 5: Non-Correlated Subquery in WHERE

```sql
-- Find products with prices above the average
SELECT 
    id,
    name,
    price,
    category
FROM products
WHERE price > (
    SELECT AVG(price) 
    FROM products 
    WHERE active = true
)
ORDER BY price DESC;
```

#### Example 6: Correlated Subquery

```sql
-- Find customers who have spent more than the average for their region
SELECT 
    c.id,
    c.name,
    c.region,
    (SELECT SUM(total) FROM orders WHERE customer_id = c.id) AS total_spent
FROM customers c
WHERE (
    SELECT SUM(total) 
    FROM orders 
    WHERE customer_id = c.id
) > (
    SELECT AVG(region_total)
    FROM (
        SELECT customer_id, SUM(total) AS region_total
        FROM orders o2
        INNER JOIN customers c2 ON o2.customer_id = c2.id
        WHERE c2.region = c.region
        GROUP BY customer_id
    ) AS regional_spending
);
```

#### Example 7: Subquery in SELECT

```sql
-- Get order details with customer's total order count
SELECT 
    o.id,
    o.customer_id,
    o.total,
    (SELECT COUNT(*) FROM orders WHERE customer_id = o.customer_id) AS customer_order_count,
    (SELECT name FROM customers WHERE id = o.customer_id) AS customer_name
FROM orders o
WHERE o.status = 'completed'
ORDER BY o.created_at DESC
LIMIT 50;
```

#### Example 8: EXISTS Subquery

```sql
-- Find customers who have placed orders in the last 30 days
SELECT 
    id,
    name,
    email
FROM customers c
WHERE EXISTS (
    SELECT 1 
    FROM orders o 
    WHERE o.customer_id = c.id 
        AND o.created_at > NOW() - INTERVAL '30 days'
)
ORDER BY name;
```

### Window Functions with PARTITION BY

#### Example 9: ROW_NUMBER for Ranking

```sql
-- Rank products by price within each category
SELECT 
    id,
    name,
    category,
    price,
    ROW_NUMBER() OVER (PARTITION BY category ORDER BY price DESC) AS price_rank
FROM products
WHERE active = true
ORDER BY category, price_rank;
```

#### Example 10: Running Total with SUM()

```sql
-- Calculate running total of sales by date
SELECT 
    sale_date,
    daily_total,
    SUM(daily_total) OVER (ORDER BY sale_date) AS running_total,
    AVG(daily_total) OVER (ORDER BY sale_date ROWS BETWEEN 6 PRECEDING AND CURRENT ROW) AS seven_day_avg
FROM (
    SELECT 
        DATE(created_at) AS sale_date,
        SUM(total) AS daily_total
    FROM orders
    WHERE status = 'completed'
    GROUP BY DATE(created_at)
) AS daily_sales
ORDER BY sale_date;
```

#### Example 11: LAG and LEAD Functions

```sql
-- Compare each month's sales with previous and next month
SELECT 
    month,
    total_sales,
    LAG(total_sales, 1) OVER (ORDER BY month) AS previous_month,
    LEAD(total_sales, 1) OVER (ORDER BY month) AS next_month,
    total_sales - LAG(total_sales, 1) OVER (ORDER BY month) AS growth
FROM monthly_sales
ORDER BY month;
```

#### Example 12: NTILE for Quartiles

```sql
-- Divide customers into quartiles based on spending
SELECT 
    id,
    name,
    total_spent,
    NTILE(4) OVER (ORDER BY total_spent DESC) AS spending_quartile
FROM (
    SELECT 
        c.id,
        c.name,
        COALESCE(SUM(o.total), 0) AS total_spent
    FROM customers c
    LEFT JOIN orders o ON c.id = o.customer_id
    GROUP BY c.id, c.name
) AS customer_spending
ORDER BY spending_quartile, total_spent DESC;
```

### Common Table Expressions (CTEs)

#### Example 13: Basic CTE

```sql
-- Use CTE to simplify complex query
WITH active_customers AS (
    SELECT 
        id,
        name,
        email,
        region
    FROM customers
    WHERE active = true AND email IS NOT NULL
),
recent_orders AS (
    SELECT 
        customer_id,
        COUNT(*) AS order_count,
        SUM(total) AS total_spent
    FROM orders
    WHERE created_at > '2024-01-01'
    GROUP BY customer_id
)
SELECT 
    ac.id,
    ac.name,
    ac.email,
    ac.region,
    COALESCE(ro.order_count, 0) AS orders,
    COALESCE(ro.total_spent, 0) AS spent
FROM active_customers ac
LEFT JOIN recent_orders ro ON ac.id = ro.customer_id
ORDER BY spent DESC;
```

#### Example 14: Recursive CTE for Hierarchies

```sql
-- Get entire organization hierarchy starting from CEO
WITH RECURSIVE org_hierarchy AS (
    -- Base case: start with CEO
    SELECT 
        id,
        name,
        position,
        manager_id,
        1 AS level,
        name AS path
    FROM employees
    WHERE manager_id IS NULL
    
    UNION ALL
    
    -- Recursive case: get direct reports
    SELECT 
        e.id,
        e.name,
        e.position,
        e.manager_id,
        oh.level + 1,
        oh.path || ' > ' || e.name
    FROM employees e
    INNER JOIN org_hierarchy oh ON e.manager_id = oh.id
    WHERE oh.level < 10  -- Prevent infinite recursion
)
SELECT 
    level,
    name,
    position,
    path
FROM org_hierarchy
ORDER BY level, name;
```

#### Example 15: Multiple CTEs

```sql
-- Calculate customer lifetime value with multiple CTEs
WITH customer_orders AS (
    SELECT 
        customer_id,
        COUNT(*) AS order_count,
        SUM(total) AS total_revenue,
        MIN(created_at) AS first_order,
        MAX(created_at) AS last_order
    FROM orders
    WHERE status = 'completed'
    GROUP BY customer_id
),
customer_metrics AS (
    SELECT 
        customer_id,
        order_count,
        total_revenue,
        total_revenue / NULLIF(order_count, 0) AS avg_order_value,
        EXTRACT(DAYS FROM (last_order - first_order)) AS customer_age_days
    FROM customer_orders
),
customer_segments AS (
    SELECT 
        customer_id,
        order_count,
        total_revenue,
        avg_order_value,
        customer_age_days,
        CASE 
            WHEN order_count >= 10 AND total_revenue > 1000 THEN 'VIP'
            WHEN order_count >= 5 THEN 'Regular'
            ELSE 'New'
        END AS segment
    FROM customer_metrics
)
SELECT 
    c.id,
    c.name,
    c.email,
    cs.order_count,
    cs.total_revenue,
    cs.avg_order_value,
    cs.customer_age_days,
    cs.segment
FROM customers c
INNER JOIN customer_segments cs ON c.id = cs.customer_id
ORDER BY cs.total_revenue DESC;
```

### CASE WHEN Expressions

#### Example 16: Simple CASE Expression

```sql
-- Categorize products by price range
SELECT 
    id,
    name,
    price,
    CASE 
        WHEN price < 10 THEN 'Budget'
        WHEN price >= 10 AND price < 50 THEN 'Mid-Range'
        WHEN price >= 50 AND price < 200 THEN 'Premium'
        ELSE 'Luxury'
    END AS price_category,
    CASE 
        WHEN stock > 100 THEN 'In Stock'
        WHEN stock > 0 THEN 'Low Stock'
        ELSE 'Out of Stock'
    END AS availability
FROM products
ORDER BY price;
```

#### Example 17: CASE in Aggregation

```sql
-- Count orders by status category
SELECT 
    EXTRACT(YEAR FROM created_at) AS year,
    EXTRACT(MONTH FROM created_at) AS month,
    COUNT(*) AS total_orders,
    COUNT(CASE WHEN status = 'completed' THEN 1 END) AS completed,
    COUNT(CASE WHEN status = 'pending' THEN 1 END) AS pending,
    COUNT(CASE WHEN status = 'cancelled' THEN 1 END) AS cancelled,
    SUM(CASE WHEN status = 'completed' THEN total ELSE 0 END) AS revenue
FROM orders
GROUP BY EXTRACT(YEAR FROM created_at), EXTRACT(MONTH FROM created_at)
ORDER BY year DESC, month DESC;
```

#### Example 18: Nested CASE Expressions

```sql
-- Complex customer scoring
SELECT 
    id,
    name,
    CASE 
        WHEN total_orders > 20 THEN
            CASE 
                WHEN avg_order_value > 100 THEN 'Platinum'
                WHEN avg_order_value > 50 THEN 'Gold'
                ELSE 'Silver'
            END
        WHEN total_orders > 10 THEN 'Bronze'
        ELSE 'Standard'
    END AS loyalty_tier,
    CASE 
        WHEN days_since_last_order IS NULL THEN 'Never Ordered'
        WHEN days_since_last_order <= 30 THEN 'Active'
        WHEN days_since_last_order <= 90 THEN 'At Risk'
        ELSE 'Inactive'
    END AS engagement_status
FROM customer_summary
ORDER BY loyalty_tier, engagement_status;
```

---

## Neuromorphic Extensions

NeuroQuantumDB extends standard SQL with neuromorphic computing concepts inspired by biological neural networks.

### NEUROMATCH - Fuzzy Pattern Matching

NEUROMATCH uses synaptic weights and activation thresholds for fuzzy, brain-like pattern matching.

#### Example 19: Basic NEUROMATCH Query

```sql
-- Find products similar to a search pattern with synaptic weighting
SELECT * FROM products 
NEUROMATCH 'wireless headphones' 
STRENGTH > 0.7;
```
**Explanation**: Matches products where the pattern similarity exceeds 0.7 (70%). Unlike LIKE, NEUROMATCH uses semantic similarity and fuzzy matching.

#### Example 20: NEUROMATCH with Learning Rate

```sql
-- Search memories with adaptive learning
SELECT 
    id,
    content,
    timestamp
FROM memories 
NEUROMATCH 'happy childhood vacation' 
STRENGTH > 0.6
LEARNING_RATE 0.01
HEBBIAN_STRENGTHENING true;
```
**Explanation**: The neural network adapts its weights during the query based on matches found. HEBBIAN_STRENGTHENING enables automatic weight reinforcement for frequently accessed patterns.

#### Example 21: NEUROMATCH with Threshold

```sql
-- User search with activation threshold
SELECT 
    user_id,
    username,
    profile_bio
FROM users
NEUROMATCH 'software engineer python machine learning'
STRENGTH > 0.5
ACTIVATION_THRESHOLD 0.8;
```
**Explanation**: ACTIVATION_THRESHOLD sets the minimum activation level required for a neuron to fire, allowing more precise control over match sensitivity.

#### Example 22: Multiple NEUROMATCH Conditions

```sql
-- Complex pattern matching across multiple fields
SELECT 
    id,
    title,
    description,
    tags
FROM articles
WHERE 
    (NEUROMATCH title 'artificial intelligence' STRENGTH > 0.7)
    OR (NEUROMATCH description 'neural networks deep learning' STRENGTH > 0.6)
    AND publish_date > '2024-01-01'
ORDER BY relevance_score DESC
LIMIT 20;
```
**Explanation**: Combines multiple NEUROMATCH conditions with standard SQL WHERE clauses for sophisticated content discovery.

### SYNAPTIC_WEIGHT Function

The `SYNAPTIC_WEIGHT` function calculates the neuromorphic similarity between a column value and a pattern string. It returns a floating-point value between 0.0 and 1.0, representing the strength of the synaptic connection (match quality).

#### Example 22a: Basic SYNAPTIC_WEIGHT in SELECT

```sql
-- Find users and their similarity to search pattern
SELECT 
    name, 
    SYNAPTIC_WEIGHT(name, 'John') AS weight 
FROM users;
```
**Explanation**: Returns all users with a weight column showing how similar each name is to 'John'. Names like 'John Doe' will have high weights (close to 1.0), while 'Jane' will have lower weights.

#### Example 22b: SYNAPTIC_WEIGHT with ORDER BY

```sql
-- Rank users by similarity to a pattern
SELECT 
    name, 
    email,
    SYNAPTIC_WEIGHT(name, 'John') AS similarity
FROM users
ORDER BY similarity DESC
LIMIT 10;
```
**Explanation**: Returns the top 10 users whose names most closely match 'John', ordered by similarity score.

#### Example 22c: SYNAPTIC_WEIGHT for Threshold Analysis

```sql
-- Find optimal threshold by examining weight distribution
SELECT 
    name,
    SYNAPTIC_WEIGHT(name, 'Smith') AS weight
FROM customers
WHERE SYNAPTIC_WEIGHT(name, 'Smith') > 0.3
ORDER BY weight DESC;
```
**Explanation**: Combines SYNAPTIC_WEIGHT in both SELECT and WHERE to filter and display match quality, useful for determining optimal thresholds.

### QUANTUM_SEARCH - Grover's Algorithm Search

QUANTUM_SEARCH uses quantum-inspired algorithms for faster searching through unstructured data.

#### Example 23: Basic Quantum Search

```sql
-- Fast search using Grover's algorithm
QUANTUM SEARCH users 
WHERE age > 30 AND city = 'Berlin';
```
**Explanation**: Provides O(√N) complexity instead of O(N) for traditional search, especially beneficial for large datasets.

#### Example 24: Quantum Search with Iterations

```sql
-- Optimize search with custom iterations
QUANTUM SEARCH products
WHERE category = 'electronics' AND price < 500
WITH ITERATIONS 100;
```
**Explanation**: ITERATIONS controls the number of amplitude amplification steps. More iterations can improve accuracy but increase computation time.

#### Example 25: Quantum Search with Oracle Function

```sql
-- Advanced quantum search with custom oracle
QUANTUM SEARCH logs
WHERE severity = 'error'
WITH ORACLE 'custom_error_detector'
AMPLITUDE_AMPLIFICATION true;
```
**Explanation**: Oracle functions define custom search criteria. AMPLITUDE_AMPLIFICATION enhances the probability of finding matching states.

### Synaptic Optimization

#### Example 26: Optimize Query with Synaptic Network

```sql
-- Let the synaptic network optimize query execution
SYNAPTIC OPTIMIZE
SELECT 
    o.id,
    c.name,
    p.product_name,
    o.total
FROM orders o
JOIN customers c ON o.customer_id = c.id
JOIN products p ON o.product_id = p.id
WHERE o.created_at > '2024-01-01'
WITH LEARNING_RATE 0.05;
```
**Explanation**: The database learns optimal execution paths through repeated queries, adapting join orders and index usage based on data patterns.

### Pattern Learning

#### Example 27: Learn User Behavior Patterns

```sql
-- Train network on user interaction patterns
LEARN PATTERN 'user_preferences'
FROM user_interactions
ALGORITHM HebbianLearning
TRAINING_EPOCHS 50;
```
**Explanation**: Extracts patterns from historical data that can be used for predictions and recommendations.

#### Example 28: Adapt Weights Based on Usage

```sql
-- Adapt synaptic weights based on query patterns
ADAPT WEIGHTS
RULE STDP
LEARNING_RATE 0.01;
```
**Explanation**: STDP (Spike-Timing Dependent Plasticity) adjusts connection weights based on the timing of neural activations, improving pattern recognition over time.

### PATTERN_MATCH vs LIKE Comparison

#### Example 29: Traditional LIKE

```sql
-- Traditional pattern matching (exact string matching)
SELECT * FROM products
WHERE name LIKE '%headphone%'
   OR name LIKE '%headset%'
   OR name LIKE '%earphone%';
```

#### Example 30: NEUROMATCH Alternative

```sql
-- Neuromorphic fuzzy matching (semantic similarity)
SELECT * FROM products
NEUROMATCH 'headphone audio listening device'
STRENGTH > 0.65;
```
**Comparison**: NEUROMATCH finds semantically similar items even without exact keyword matches, understanding that "wireless earbuds" and "bluetooth headset" relate to "headphones".

### Combined Neuromorphic Queries

#### Example 31: Hybrid Quantum-Neural Query

```sql
-- Combine quantum search with neural matching
WITH quantum_results AS (
    QUANTUM SEARCH products
    WHERE category = 'electronics'
    WITH ITERATIONS 80
)
SELECT 
    p.*,
    similarity_score
FROM quantum_results qr
JOIN products p ON qr.id = p.id
WHERE NEUROMATCH p.description 'high quality premium' STRENGTH > 0.7
ORDER BY similarity_score DESC, p.price ASC
LIMIT 10;
```
**Explanation**: Leverages quantum search for fast filtering, then applies neural pattern matching for relevance ranking.

#### Example 32: Adaptive Recommendation System

```sql
-- Product recommendations using learned patterns
WITH user_profile AS (
    SELECT pattern_vector 
    FROM learned_patterns 
    WHERE pattern_name = 'user_preferences' 
        AND user_id = 12345
)
SELECT 
    p.id,
    p.name,
    p.price,
    synaptic_similarity(p.features, up.pattern_vector) AS match_score
FROM products p
CROSS JOIN user_profile up
WHERE NEUROMATCH p.description user_search_query STRENGTH > 0.6
ORDER BY match_score DESC
LIMIT 20;
```
**Explanation**: Combines learned user preferences with real-time neural matching for personalized recommendations.

---

## Performance Tips

### Index Usage

#### Example 33: Create Strategic Indexes

```sql
-- Create index for frequently queried columns
CREATE INDEX idx_orders_customer_created 
ON orders(customer_id, created_at DESC);

-- Create partial index for active records only
CREATE INDEX idx_active_products 
ON products(category, price) 
WHERE active = true;

-- Create covering index to avoid table lookups
CREATE INDEX idx_customer_covering 
ON customers(region, name, email);
```
**Tip**: Use EXPLAIN to verify index usage:
```sql
EXPLAIN SELECT * FROM orders 
WHERE customer_id = 123 
ORDER BY created_at DESC;
```

### Query Optimization

#### Example 34: Optimize with EXISTS instead of IN

```sql
-- Less efficient: IN with subquery
SELECT * FROM customers
WHERE id IN (
    SELECT customer_id FROM orders WHERE total > 1000
);

-- More efficient: EXISTS
SELECT * FROM customers c
WHERE EXISTS (
    SELECT 1 FROM orders o 
    WHERE o.customer_id = c.id AND o.total > 1000
);
```
**Tip**: EXISTS can short-circuit after finding first match, while IN must evaluate all results.

#### Example 35: Use Column Projection

```sql
-- Inefficient: selecting all columns
SELECT * FROM large_table WHERE id = 123;

-- Efficient: select only needed columns
SELECT id, name, email FROM large_table WHERE id = 123;
```
**Tip**: Reduce data transfer and memory usage by selecting only required columns.

### Batch Operations

#### Example 36: Batch Inserts

```sql
-- Efficient batch insert
INSERT INTO logs (timestamp, level, message) VALUES
    ('2024-01-07 10:00:00', 'INFO', 'Service started'),
    ('2024-01-07 10:00:01', 'INFO', 'Connection established'),
    ('2024-01-07 10:00:02', 'DEBUG', 'Processing request'),
    ('2024-01-07 10:00:03', 'INFO', 'Request completed');
```
**Tip**: Batch operations reduce transaction overhead. Aim for 100-1000 rows per batch.

#### Example 37: Batch Updates with CASE

```sql
-- Update multiple records efficiently
UPDATE products
SET price = CASE id
    WHEN 1 THEN 19.99
    WHEN 2 THEN 29.99
    WHEN 3 THEN 39.99
    WHEN 4 THEN 49.99
    ELSE price
END,
stock = CASE id
    WHEN 1 THEN stock - 5
    WHEN 2 THEN stock - 3
    WHEN 3 THEN stock - 7
    WHEN 4 THEN stock - 2
    ELSE stock
END
WHERE id IN (1, 2, 3, 4);
```
**Tip**: Single UPDATE statement is more efficient than multiple individual updates.

---

## Real-World Use Cases

### User Search with Fuzzy Matching

#### Example 38: Intelligent User Search

```sql
-- Search for users with typo tolerance and semantic matching
WITH search_results AS (
    SELECT 
        id,
        username,
        full_name,
        bio,
        location
    FROM users
    WHERE NEUROMATCH (username || ' ' || full_name || ' ' || bio) 
                     'jon smith software engineer' 
          STRENGTH > 0.5
)
SELECT 
    sr.*,
    (
        -- Bonus score for location match
        CASE WHEN location ILIKE '%san francisco%' THEN 0.2 ELSE 0 END +
        -- Bonus score for verified users
        CASE WHEN verified = true THEN 0.1 ELSE 0 END
    ) AS bonus_score
FROM search_results sr
ORDER BY (STRENGTH + bonus_score) DESC
LIMIT 20;
```
**Use Case**: User directory search that handles typos, variations, and semantic similarity.

### Product Recommendations

#### Example 39: Collaborative Filtering Recommendations

```sql
-- Find products bought by similar users
WITH user_purchases AS (
    SELECT product_id
    FROM orders
    WHERE customer_id = 12345 AND status = 'completed'
),
similar_users AS (
    SELECT DISTINCT o.customer_id
    FROM orders o
    WHERE o.product_id IN (SELECT product_id FROM user_purchases)
        AND o.customer_id != 12345
        AND o.status = 'completed'
),
recommended_products AS (
    SELECT 
        p.id,
        p.name,
        p.price,
        COUNT(DISTINCT o.customer_id) AS purchase_count,
        AVG(o.total) AS avg_order_value
    FROM products p
    JOIN orders o ON p.id = o.product_id
    WHERE o.customer_id IN (SELECT customer_id FROM similar_users)
        AND p.id NOT IN (SELECT product_id FROM user_purchases)
        AND p.active = true
    GROUP BY p.id, p.name, p.price
    HAVING COUNT(DISTINCT o.customer_id) >= 3
)
SELECT *
FROM recommended_products
ORDER BY purchase_count DESC, avg_order_value DESC
LIMIT 10;
```
**Use Case**: E-commerce product recommendations based on similar users' purchases.

### Anomaly Detection

#### Example 40: Detect Unusual Patterns

```sql
-- Identify abnormal transaction patterns
WITH transaction_stats AS (
    SELECT 
        customer_id,
        AVG(total) AS avg_transaction,
        STDDEV(total) AS stddev_transaction,
        COUNT(*) AS transaction_count
    FROM orders
    WHERE created_at > NOW() - INTERVAL '90 days'
    GROUP BY customer_id
    HAVING COUNT(*) >= 5
),
recent_transactions AS (
    SELECT 
        o.id,
        o.customer_id,
        o.total,
        o.created_at
    FROM orders o
    WHERE o.created_at > NOW() - INTERVAL '7 days'
)
SELECT 
    rt.id AS order_id,
    rt.customer_id,
    rt.total AS transaction_amount,
    ts.avg_transaction AS typical_amount,
    rt.created_at,
    -- Z-score: how many standard deviations from mean
    (rt.total - ts.avg_transaction) / NULLIF(ts.stddev_transaction, 0) AS z_score,
    CASE 
        WHEN ABS((rt.total - ts.avg_transaction) / NULLIF(ts.stddev_transaction, 0)) > 3 
        THEN 'High Risk'
        WHEN ABS((rt.total - ts.avg_transaction) / NULLIF(ts.stddev_transaction, 0)) > 2 
        THEN 'Medium Risk'
        ELSE 'Normal'
    END AS risk_level
FROM recent_transactions rt
JOIN transaction_stats ts ON rt.customer_id = ts.customer_id
WHERE ABS((rt.total - ts.avg_transaction) / NULLIF(ts.stddev_transaction, 0)) > 2
ORDER BY z_score DESC;
```
**Use Case**: Fraud detection by identifying transactions that deviate significantly from a customer's normal behavior.

### Time-Series Analysis

#### Example 41: Sales Trend Analysis

```sql
-- Comprehensive sales trend analysis with seasonality
WITH daily_sales AS (
    SELECT 
        DATE(created_at) AS sale_date,
        SUM(total) AS daily_revenue,
        COUNT(*) AS order_count,
        AVG(total) AS avg_order_value
    FROM orders
    WHERE status = 'completed'
        AND created_at >= NOW() - INTERVAL '365 days'
    GROUP BY DATE(created_at)
),
sales_with_trends AS (
    SELECT 
        sale_date,
        daily_revenue,
        order_count,
        avg_order_value,
        -- 7-day moving average
        AVG(daily_revenue) OVER (
            ORDER BY sale_date 
            ROWS BETWEEN 6 PRECEDING AND CURRENT ROW
        ) AS ma_7day,
        -- 30-day moving average
        AVG(daily_revenue) OVER (
            ORDER BY sale_date 
            ROWS BETWEEN 29 PRECEDING AND CURRENT ROW
        ) AS ma_30day,
        -- Week-over-week growth
        daily_revenue - LAG(daily_revenue, 7) OVER (ORDER BY sale_date) AS wow_change,
        -- Month-over-month comparison
        daily_revenue - LAG(daily_revenue, 30) OVER (ORDER BY sale_date) AS mom_change
    FROM daily_sales
)
SELECT 
    sale_date,
    daily_revenue,
    order_count,
    avg_order_value,
    ma_7day,
    ma_30day,
    wow_change,
    mom_change,
    CASE 
        WHEN wow_change > 0 THEN 'Growing'
        WHEN wow_change < 0 THEN 'Declining'
        ELSE 'Stable'
    END AS trend,
    -- Identify day of week pattern
    EXTRACT(DOW FROM sale_date) AS day_of_week,
    CASE EXTRACT(DOW FROM sale_date)
        WHEN 0 THEN 'Sunday'
        WHEN 1 THEN 'Monday'
        WHEN 2 THEN 'Tuesday'
        WHEN 3 THEN 'Wednesday'
        WHEN 4 THEN 'Thursday'
        WHEN 5 THEN 'Friday'
        WHEN 6 THEN 'Saturday'
    END AS day_name
FROM sales_with_trends
WHERE sale_date >= NOW() - INTERVAL '90 days'
ORDER BY sale_date DESC;
```
**Use Case**: Business intelligence dashboard for sales performance tracking with trend identification.

#### Example 42: Time-Series Forecasting with Neural Learning

```sql
-- Train pattern for sales forecasting
LEARN PATTERN 'sales_seasonality'
FROM (
    SELECT 
        EXTRACT(DOW FROM created_at) AS day_of_week,
        EXTRACT(HOUR FROM created_at) AS hour_of_day,
        EXTRACT(MONTH FROM created_at) AS month,
        COUNT(*) AS order_volume,
        SUM(total) AS revenue
    FROM orders
    WHERE created_at >= NOW() - INTERVAL '365 days'
    GROUP BY 
        EXTRACT(DOW FROM created_at),
        EXTRACT(HOUR FROM created_at),
        EXTRACT(MONTH FROM created_at)
) AS historical_patterns
ALGORITHM UnsupervisedClustering
TRAINING_EPOCHS 100;

-- Use learned pattern for prediction
SELECT 
    predicted_revenue,
    confidence_score
FROM predict_pattern('sales_seasonality', 
    day_of_week => 5, 
    hour_of_day => 14, 
    month => 1
);
```
**Use Case**: Predictive analytics for inventory planning and staffing optimization.

---

## Summary

This guide covered **42 comprehensive examples** including:

- **18 Standard SQL examples**: JOINs, subqueries, window functions, CTEs, CASE expressions
- **14 Neuromorphic extension examples**: NEUROMATCH, QUANTUM_SEARCH, synaptic optimization, pattern learning
- **5 Performance optimization examples**: Index usage, query optimization, batch operations
- **5 Real-world use case examples**: User search, recommendations, anomaly detection, time-series analysis

### Key Takeaways

1. **NEUROMATCH** provides fuzzy, semantic matching superior to LIKE for natural language queries
2. **QUANTUM_SEARCH** offers performance benefits for large-scale unstructured searches
3. **Synaptic optimization** allows the database to learn and adapt query execution strategies
4. **Pattern learning** enables predictive analytics and intelligent recommendations
5. **Proper indexing and query structure** are crucial for performance regardless of features used

### Next Steps

- [QSQL Query Language Reference](qsql.md) - Complete syntax reference
- [Feature Documentation](features.md) - Detailed feature guides
- [REST API](rest-api.md) - HTTP API documentation
- [Performance Benchmarks](../reference/benchmarks.md) - Performance characteristics

For more examples and community contributions, visit the [GitHub repository](https://github.com/neuroquantumdb/neuroquantumdb).
