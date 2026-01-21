# NeuroQuantumDB PHP Driver

[![PHP Version](https://img.shields.io/badge/PHP-8.2%2B-blue.svg)](https://www.php.net/)
[![License](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

Official PHP driver for **NeuroQuantumDB** - A neuromorphic and quantum-inspired database. This library provides a secure, type-safe, and performant way to interact with NeuroQuantumDB's REST API.

## Features

- 🔒 **Secure Configuration** - API keys stored safely via environment variables or encrypted config
- 🚀 **Fluent Query Builder** - Laravel-like query interface for SELECT, INSERT, UPDATE, DELETE
- 🔄 **Automatic Retries** - Built-in retry logic with exponential backoff
- 📝 **Full Type Safety** - PHP 8.2+ with strict types and PHPDoc annotations
- ⚡ **Lazy Loading** - Specialized clients loaded only when needed
- 🧪 **Comprehensive Testing** - PHPUnit tests with mocked HTTP responses
- 🔬 **Advanced Features** - Quantum search, neural networks, DNA compression, biometric auth

## Requirements

- PHP 8.2 or higher
- Composer
- cURL extension (for Guzzle)
- JSON extension

## Installation

```bash
composer require neuroquantum/php-driver
```

## Quick Start

### 1. Configuration

Create a `.env` file in your project root:

```env
NEUROQUANTUM_HOST=localhost
NEUROQUANTUM_PORT=8080
NEUROQUANTUM_API_KEY=nqdb_your_api_key_here
NEUROQUANTUM_USE_SSL=false
```

### 2. Connect to Database

```php
<?php

use NeuroQuantum\NeuroQuantumDB;

// Connect using environment variables
$db = NeuroQuantumDB::connect();

// Or with explicit configuration
$db = NeuroQuantumDB::create([
    'host' => 'localhost',
    'port' => 8080,
    'api_key' => 'nqdb_your_api_key',
    'ssl' => false,
    'timeout' => 30,
]);
```

### 3. Execute Queries

```php
// Raw SQL query
$result = $db->query("SELECT * FROM users WHERE age > 21");
foreach ($result->rows as $user) {
    echo $user['name'] . "\n";
}

// Fluent query builder
$users = $db->table('users')
    ->select(['id', 'name', 'email'])
    ->where('age', '>', 21)
    ->whereNotNull('email_verified_at')
    ->orderBy('name')
    ->limit(10)
    ->get();
```

## API Reference

### NeuroQuantumDB Class

Main entry point providing access to all database functionality.

#### Connection Methods

```php
// Connect with environment variables
$db = NeuroQuantumDB::connect();

// Connect with array config
$db = NeuroQuantumDB::create(['host' => '...', 'api_key' => '...']);

// Connect with Config object
$db = NeuroQuantumDB::createFromConfig($config);

// Lazy connect (for testing)
$db = NeuroQuantumDB::createLazy($config);

// Check connection status
$db->isConnected(); // bool
```

#### Raw Query Methods

```php
// Execute any QSQL query
$result = $db->query("SELECT * FROM users");

// Get rows directly
$rows = $db->select("SELECT * FROM users");

// Get single row
$user = $db->selectOne("SELECT * FROM users WHERE id = 1");

// Execute INSERT/UPDATE/DELETE and get affected rows
$count = $db->execute("DELETE FROM sessions WHERE expired = true");
```

#### Table Operations

```php
use NeuroQuantum\Client\TableSchema;

// Create table
$schema = TableSchema::create('users')
    ->id()
    ->string('name')
    ->string('email')
    ->boolean('is_active')
    ->timestamps()
    ->unique('email');

$db->createTable($schema, ifNotExists: true);

// Drop table
$db->dropTable('users', ifExists: true);

// Truncate table
$db->truncateTable('users');

// Insert records
$result = $db->insert('users', [
    ['name' => 'Alice', 'email' => 'alice@example.com'],
    ['name' => 'Bob', 'email' => 'bob@example.com'],
]);
echo "Inserted: " . $result->insertedCount;

// Update records
$affected = $db->update('users', 
    ['status' => 'inactive'],
    ['last_login' => null]
);

// Delete records
$deleted = $db->delete('users', ['status' => 'deleted']);

// Find by ID
$user = $db->find('users', 1);
```

#### Fluent Query Builder

```php
// SELECT queries
$users = $db->table('users')
    ->select(['id', 'name', 'email'])
    ->where('status', 'active')
    ->where('age', '>=', 18)
    ->orWhere('role', 'admin')
    ->whereIn('department', ['IT', 'HR'])
    ->whereNotNull('verified_at')
    ->whereLike('email', '%@company.com')
    ->orderBy('name')
    ->orderByDesc('created_at')
    ->limit(50)
    ->offset(100)
    ->get();

// Pagination
$page = $db->table('users')
    ->paginate(page: 3, perPage: 25)
    ->get();

// Aggregates
$count = $db->table('users')->where('active', true)->count();
$exists = $db->table('users')->where('email', 'test@example.com')->exists();

// Single value
$email = $db->table('users')->where('id', 1)->value('email');

// Pluck column
$names = $db->table('users')->where('active', true)->pluck('name');

// First record
$user = $db->table('users')->where('id', 1)->first();

// INSERT via builder
$result = $db->table('users')
    ->insertOne(['name' => 'Charlie', 'email' => 'charlie@example.com']);

// UPDATE via builder
$affected = $db->table('users')
    ->where('id', 1)
    ->updateSet(['name' => 'Updated Name']);

// Increment/Decrement
$db->table('products')->where('id', 1)->increment('views');
$db->table('products')->where('id', 1)->decrement('stock', 5);

// DELETE via builder
$deleted = $db->table('users')
    ->where('status', 'deleted')
    ->deleteRows();
```

#### Transactions

```php
// Manual transaction control
$db->beginTransaction();
try {
    $db->insert('orders', [['user_id' => 1, 'total' => 99.99]]);
    $db->update('users', ['order_count' => 1], ['id' => 1]);
    $db->commit();
} catch (Exception $e) {
    $db->rollback();
    throw $e;
}

// Transaction with callback
$result = $db->transaction(function () use ($db) {
    $db->insert('orders', [['user_id' => 1, 'total' => 99.99]]);
    return $db->find('orders', 1);
});
```

### Specialized Clients

#### Quantum Search

```php
$quantum = $db->quantum();

// Similarity search
$results = $quantum->search(
    table: 'embeddings',
    queryVector: [0.1, 0.5, 0.8, 0.3],
    threshold: 0.7,
    maxResults: 10
);

// Grover's algorithm search
$results = $quantum->groverSearch(
    table: 'embeddings',
    queryVector: [0.1, 0.5, 0.8, 0.3],
    numShots: 1024,
    errorMitigation: true
);
```

#### Neural Networks

```php
$neural = $db->neural();

// Train model
$jobId = $neural->train('my_model', $trainingData, [
    'epochs' => 100,
    'learning_rate' => 0.01,
]);

// Check status
$status = $neural->getTrainingStatus($jobId);

// Wait for completion
$result = $neural->waitForTraining($jobId, timeoutSeconds: 300);
```

#### DNA Compression

```php
$dna = $db->dna();

// Compress DNA sequences
$result = $dna->compress(
    sequences: ['ATCGATCGATCG', 'GCTAGCTAGCTA'],
    algorithm: DnaClient::ALGORITHM_KMER,
    compressionLevel: 5
);

// Decompress
$sequences = $dna->decompress($compressedData);
```

#### Biometric Authentication

```php
$bio = $db->biometric();

// Enroll user
$bio->enroll('user123', $biometricVector, 'eeg');

// Verify
$result = $bio->verify('user123', $biometricVector);
if ($result['match']) {
    echo "Authenticated with score: " . $result['score'];
}

// EEG authentication
$authResult = $bio->authenticateEeg('user123', $eegData);
```

#### Statistics & Monitoring

```php
$stats = $db->stats();

// Get performance stats
$perf = $stats->getPerformance();

// Get detailed metrics (admin)
$metrics = $stats->getMetrics();

// Get index recommendations
$recommendations = $stats->getIndexRecommendations();
```

#### API Key Management (Admin)

```php
$auth = $db->auth();

// Generate new API key
$keyData = $auth->generateApiKey(
    name: 'my-app',
    permissions: ['read', 'write'],
    expiryHours: 720,
    rateLimitPerHour: 1000
);
echo "New key: " . $keyData['api_key'];

// Revoke key
$auth->revokeApiKey('nqdb_old_key_to_revoke');
```

### Query Builders

#### SelectBuilder

```php
use NeuroQuantum\Query\SelectBuilder;
use NeuroQuantum\Query\FilterOperator;
use NeuroQuantum\Query\SortDirection;

$builder = SelectBuilder::from('users')
    ->select(['id', 'name', 'email'])
    ->where('age', FilterOperator::GreaterThan, 21)
    ->whereIn('status', ['active', 'pending'])
    ->whereNull('deleted_at')
    ->orderBy('name', SortDirection::Asc)
    ->limit(10)
    ->offset(0);

// Get SQL string
$sql = $builder->toSql();

// Get API array format
$apiRequest = $builder->toArray();
```

#### InsertBuilder

```php
use NeuroQuantum\Query\InsertBuilder;

$builder = InsertBuilder::into('users')
    ->values(['name' => 'John', 'email' => 'john@example.com'])
    ->values(['name' => 'Jane', 'email' => 'jane@example.com'])
    ->batchSize(1000)
    ->orIgnore();
```

#### UpdateBuilder

```php
use NeuroQuantum\Query\UpdateBuilder;

$builder = UpdateBuilder::table('users')
    ->set('name', 'New Name')
    ->setMany(['email' => 'new@email.com', 'status' => 'updated'])
    ->increment('login_count')
    ->where('id', 1);
```

#### DeleteBuilder

```php
use NeuroQuantum\Query\DeleteBuilder;

$builder = DeleteBuilder::from('users')
    ->where('status', 'inactive')
    ->whereNull('last_login');
```

### Table Schema Definition

```php
use NeuroQuantum\Client\TableSchema;

$schema = TableSchema::create('products')
    ->id()                              // Auto-increment primary key
    ->string('name')                    // VARCHAR/TEXT
    ->text('description', nullable: true)
    ->float('price')
    ->integer('quantity')
    ->boolean('is_active')
    ->timestamp('published_at', nullable: true)
    ->json('metadata', nullable: true)
    ->binary('image', nullable: true)
    ->dna('sequence', nullable: true)   // DNA sequence type
    ->neural('embedding', nullable: true) // Neural vector type
    ->quantum('state', nullable: true)  // Quantum state type
    ->timestamps()                      // created_at, updated_at
    ->unique('name')
    ->index(['category_id', 'price']);
```

### Filter Operators

| Operator | PHP Enum | SQL Equivalent |
|----------|----------|----------------|
| `Equals` | `FilterOperator::Equals` | `=` |
| `NotEquals` | `FilterOperator::NotEquals` | `!=` |
| `GreaterThan` | `FilterOperator::GreaterThan` | `>` |
| `LessThan` | `FilterOperator::LessThan` | `<` |
| `GreaterThanOrEquals` | `FilterOperator::GreaterThanOrEquals` | `>=` |
| `LessThanOrEquals` | `FilterOperator::LessThanOrEquals` | `<=` |
| `In` | `FilterOperator::In` | `IN (...)` |
| `NotIn` | `FilterOperator::NotIn` | `NOT IN (...)` |
| `Like` | `FilterOperator::Like` | `LIKE` |
| `NotLike` | `FilterOperator::NotLike` | `NOT LIKE` |
| `IsNull` | `FilterOperator::IsNull` | `IS NULL` |
| `IsNotNull` | `FilterOperator::IsNotNull` | `IS NOT NULL` |
| `Contains` | `FilterOperator::Contains` | `LIKE '%...%'` |
| `StartsWith` | `FilterOperator::StartsWith` | `LIKE '...%'` |
| `EndsWith` | `FilterOperator::EndsWith` | `LIKE '%...'` |
| `NeuralSimilarity` | `FilterOperator::NeuralSimilarity` | Neural matching |
| `QuantumEntanglement` | `FilterOperator::QuantumEntanglement` | Quantum search |

### Exception Handling

```php
use NeuroQuantum\Exception\{
    NeuroQuantumException,
    AuthenticationException,
    AuthorizationException,
    ValidationException,
    QueryException,
    NotFoundException,
    ConflictException,
    RateLimitException,
    ConnectionException,
    ServerException
};

try {
    $db->query("SELECT * FROM users");
} catch (AuthenticationException $e) {
    // Invalid or expired API key (401)
    echo "Auth failed: " . $e->getMessage();
} catch (AuthorizationException $e) {
    // Insufficient permissions (403)
    echo "Permission denied: " . $e->getMessage();
} catch (ValidationException $e) {
    // Validation errors (400)
    foreach ($e->getErrors() as $field => $errors) {
        echo "$field: " . implode(', ', $errors) . "\n";
    }
} catch (QueryException $e) {
    // SQL syntax or execution error
    echo "Query failed: " . $e->getMessage();
    echo "Query: " . $e->getQuery();
} catch (NotFoundException $e) {
    // Resource not found (404)
    echo "Not found: " . $e->getMessage();
} catch (ConflictException $e) {
    // Conflict (409) - e.g., duplicate key
    echo "Conflict: " . $e->getMessage();
} catch (RateLimitException $e) {
    // Rate limit exceeded (429)
    echo "Rate limited. Retry after: " . $e->getRetryAfter() . "s";
} catch (ConnectionException $e) {
    // Connection failed
    echo "Connection error: " . $e->getMessage();
} catch (ServerException $e) {
    // Server error (5xx)
    echo "Server error: " . $e->getMessage();
} catch (NeuroQuantumException $e) {
    // Any other error
    echo "Error: " . $e->getMessage();
    print_r($e->getContext());
}
```

## Configuration Options

| Option | Environment Variable | Default | Description |
|--------|---------------------|---------|-------------|
| `host` | `NEUROQUANTUM_HOST` | `localhost` | Database host |
| `port` | `NEUROQUANTUM_PORT` | `8080` | Database port |
| `api_key` | `NEUROQUANTUM_API_KEY` | (required) | API authentication key |
| `ssl` | `NEUROQUANTUM_USE_SSL` | `false` | Use HTTPS |
| `timeout` | `NEUROQUANTUM_TIMEOUT` | `30` | Connection timeout (seconds) |
| `retry_enabled` | `NEUROQUANTUM_RETRY_ENABLED` | `true` | Enable automatic retries |
| `max_retry_attempts` | `NEUROQUANTUM_RETRY_MAX_ATTEMPTS` | `3` | Maximum retry attempts |
| `debug` | `NEUROQUANTUM_DEBUG` | `false` | Enable debug logging |

## Testing

Run the test suite:

```bash
# Run all tests
composer test

# Run with coverage
composer test:coverage

# Run static analysis
composer analyse

# Check code style
composer cs

# Run all checks
composer check
```

## Security Best Practices

1. **Never commit API keys** - Use environment variables or secure vaults
2. **Use HTTPS in production** - Set `NEUROQUANTUM_USE_SSL=true`
3. **Validate user input** - Use parameterized queries via builders
4. **Limit permissions** - Use API keys with minimal required permissions
5. **Rotate keys regularly** - Generate new keys and revoke old ones

## Contributing

Contributions are welcome! Please read our contributing guidelines before submitting pull requests.

## License

This library is licensed under the MIT License. See [LICENSE](LICENSE) for details.
