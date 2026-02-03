<?php

/**
 * NeuroQuantumDB PHP Driver
 *
 * Official PHP driver for NeuroQuantumDB - A high-performance neuromorphic
 * and quantum-inspired database.
 *
 * @package NeuroQuantum
 * @version 0.0.0
 * @see CHANGELOG.md for release history
 */

declare(strict_types=1);

namespace NeuroQuantum;

use NeuroQuantum\Client\AuthClient;
use NeuroQuantum\Client\BiometricClient;
use NeuroQuantum\Client\DnaClient;
use NeuroQuantum\Client\InsertResult;
use NeuroQuantum\Client\NeuralClient;
use NeuroQuantum\Client\QuantumClient;
use NeuroQuantum\Client\QueryClient;
use NeuroQuantum\Client\QueryResult;
use NeuroQuantum\Client\StatsClient;
use NeuroQuantum\Client\TableClient;
use NeuroQuantum\Client\TableSchema;
use NeuroQuantum\Config\Config;
use NeuroQuantum\Contract\ConfigInterface;
use NeuroQuantum\Contract\ConnectionInterface;
use NeuroQuantum\Exception\ConnectionException;
use NeuroQuantum\Http\HttpConnection;
use NeuroQuantum\Query\DeleteBuilder;
use NeuroQuantum\Query\InsertBuilder;
use NeuroQuantum\Query\SelectBuilder;
use NeuroQuantum\Query\UpdateBuilder;
use Psr\Log\LoggerInterface;
use Psr\Log\NullLogger;

/**
 * Main entry point for NeuroQuantumDB PHP driver.
 *
 * Provides a simple, fluent API to interact with NeuroQuantumDB.
 *
 * @example
 * ```php
 * // Connect using environment variables
 * $db = NeuroQuantumDB::connect();
 *
 * // Or with explicit config
 * $db = NeuroQuantumDB::create([
 *     'host' => 'localhost',
 *     'port' => 8080,
 *     'api_key' => 'nqdb_xxx',
 * ]);
 *
 * // Execute queries
 * $users = $db->query("SELECT * FROM users WHERE age > 21");
 *
 * // Use fluent query builder
 * $users = $db->table('users')
 *     ->select(['id', 'name', 'email'])
 *     ->where('age', '>', 21)
 *     ->orderBy('name')
 *     ->limit(10)
 *     ->get();
 * ```
 */
final class NeuroQuantumDB
{
    private QueryClient $queryClient;
    private TableClient $tableClient;
    private ?QuantumClient $quantumClient = null;
    private ?NeuralClient $neuralClient = null;
    private ?DnaClient $dnaClient = null;
    private ?BiometricClient $biometricClient = null;
    private ?StatsClient $statsClient = null;
    private ?AuthClient $authClient = null;

    public function __construct(
        private readonly ConnectionInterface $connection,
    ) {
        $this->queryClient = new QueryClient($this->connection);
        $this->tableClient = new TableClient($this->connection);
    }

    /**
     * Create a new NeuroQuantumDB instance from environment variables.
     *
     * @throws ConnectionException When connection fails
     */
    public static function connect(?LoggerInterface $logger = null): self
    {
        $config = Config::fromEnvironment();
        return self::createFromConfig($config, $logger);
    }

    /**
     * Create a new NeuroQuantumDB instance from configuration array.
     *
     * @param array<string, mixed> $config Configuration options
     * @throws ConnectionException When connection fails
     */
    public static function create(array $config, ?LoggerInterface $logger = null): self
    {
        $configObj = Config::fromArray($config);
        return self::createFromConfig($configObj, $logger);
    }

    /**
     * Create from ConfigInterface.
     */
    public static function createFromConfig(ConfigInterface $config, ?LoggerInterface $logger = null): self
    {
        $connection = new HttpConnection($config, $logger ?? new NullLogger());
        $connection->connect();
        return new self($connection);
    }

    /**
     * Create without verifying connection (useful for testing).
     */
    public static function createLazy(ConfigInterface $config, ?LoggerInterface $logger = null): self
    {
        $connection = new HttpConnection($config, $logger ?? new NullLogger());
        return new self($connection);
    }

    // ========================================
    // Raw Query Methods
    // ========================================

    /**
     * Execute a raw QSQL query.
     *
     * @param string $sql QSQL query string
     * @return QueryResult Query result
     */
    public function query(string $sql): QueryResult
    {
        return $this->queryClient->execute($sql);
    }

    /**
     * Execute a raw query and return rows.
     *
     * @param string $sql QSQL query string
     * @return array<int, array<string, mixed>> Result rows
     */
    public function select(string $sql): array
    {
        return $this->query($sql)->rows;
    }

    /**
     * Execute query and return first row.
     *
     * @param string $sql QSQL query string
     * @return array<string, mixed>|null First row or null
     */
    public function selectOne(string $sql): ?array
    {
        return $this->query($sql)->first();
    }

    /**
     * Execute an insert/update/delete query.
     *
     * @param string $sql QSQL statement
     * @return int Number of affected rows
     */
    public function execute(string $sql): int
    {
        return $this->query($sql)->rowsAffected;
    }

    // ========================================
    // Query Builder Methods
    // ========================================

    /**
     * Start a SELECT query builder for a table.
     *
     * @param string $table Table name
     * @return TableQueryBuilder Fluent query builder
     */
    public function table(string $table): TableQueryBuilder
    {
        return new TableQueryBuilder($this->tableClient, $table);
    }

    /**
     * Create a SELECT builder directly.
     *
     * @param string $table Table name
     */
    public function from(string $table): SelectBuilder
    {
        return SelectBuilder::from($table);
    }

    /**
     * Create an INSERT builder.
     *
     * @param string $table Table name
     */
    public function into(string $table): InsertBuilder
    {
        return InsertBuilder::into($table);
    }

    // ========================================
    // Table Operations
    // ========================================

    /**
     * Create a new table.
     *
     * @param TableSchema $schema Table schema definition
     * @param bool $ifNotExists Don't error if table exists
     */
    public function createTable(TableSchema $schema, bool $ifNotExists = false): bool
    {
        return $this->tableClient->create($schema, $ifNotExists);
    }

    /**
     * Drop a table.
     *
     * @param string $table Table name
     * @param bool $ifExists Don't error if table doesn't exist
     */
    public function dropTable(string $table, bool $ifExists = false): bool
    {
        return $this->tableClient->drop($table, $ifExists);
    }

    /**
     * Truncate a table.
     *
     * @param string $table Table name
     */
    public function truncateTable(string $table): bool
    {
        return $this->tableClient->truncate($table);
    }

    /**
     * Insert records into a table.
     *
     * @param string $table Table name
     * @param array<int, array<string, mixed>> $records Records to insert
     */
    public function insert(string $table, array $records): InsertResult
    {
        return $this->tableClient->insert($table, $records);
    }

    /**
     * Update records in a table.
     *
     * @param string $table Table name
     * @param array<string, mixed> $updates Column updates
     * @param array<string, mixed> $where Where conditions
     * @return int Number of rows updated
     */
    public function update(string $table, array $updates, array $where = []): int
    {
        $builder = UpdateBuilder::table($table)->setMany($updates);
        foreach ($where as $column => $value) {
            $builder->where($column, $value);
        }
        return $this->tableClient->updateBuilder($builder);
    }

    /**
     * Delete records from a table.
     *
     * @param string $table Table name
     * @param array<string, mixed> $where Where conditions
     * @return int Number of rows deleted
     */
    public function delete(string $table, array $where = []): int
    {
        $builder = DeleteBuilder::from($table);
        foreach ($where as $column => $value) {
            $builder->where($column, $value);
        }
        return $this->tableClient->deleteBuilder($builder);
    }

    /**
     * Find a record by ID.
     *
     * @param string $table Table name
     * @param string|int $id Record ID
     * @param string $idColumn ID column name
     * @return array<string, mixed>|null Record or null
     */
    public function find(string $table, string|int $id, string $idColumn = 'id'): ?array
    {
        return $this->tableClient->find($table, $id, $idColumn);
    }

    // ========================================
    // Transaction Methods
    // ========================================

    /**
     * Begin a transaction.
     */
    public function beginTransaction(): void
    {
        $this->queryClient->beginTransaction();
    }

    /**
     * Commit the current transaction.
     */
    public function commit(): void
    {
        $this->queryClient->commit();
    }

    /**
     * Rollback the current transaction.
     */
    public function rollback(): void
    {
        $this->queryClient->rollback();
    }

    /**
     * Execute callback within a transaction.
     *
     * @template T
     * @param callable(): T $callback
     * @return T The callback result
     */
    public function transaction(callable $callback): mixed
    {
        return $this->queryClient->transaction($callback);
    }

    // ========================================
    // Specialized Clients (Lazy Loading)
    // ========================================

    /**
     * Get quantum operations client.
     */
    public function quantum(): QuantumClient
    {
        return $this->quantumClient ??= new QuantumClient($this->connection);
    }

    /**
     * Get neural network operations client.
     */
    public function neural(): NeuralClient
    {
        return $this->neuralClient ??= new NeuralClient($this->connection);
    }

    /**
     * Get DNA compression client.
     */
    public function dna(): DnaClient
    {
        return $this->dnaClient ??= new DnaClient($this->connection);
    }

    /**
     * Get biometric authentication client.
     */
    public function biometric(): BiometricClient
    {
        return $this->biometricClient ??= new BiometricClient($this->connection);
    }

    /**
     * Get stats and monitoring client.
     */
    public function stats(): StatsClient
    {
        return $this->statsClient ??= new StatsClient($this->connection);
    }

    /**
     * Get API key management client (admin only).
     */
    public function auth(): AuthClient
    {
        return $this->authClient ??= new AuthClient($this->connection);
    }

    /**
     * Get the raw query client.
     */
    public function queries(): QueryClient
    {
        return $this->queryClient;
    }

    /**
     * Get the raw table client.
     */
    public function tables(): TableClient
    {
        return $this->tableClient;
    }

    /**
     * Get the underlying connection.
     */
    public function getConnection(): ConnectionInterface
    {
        return $this->connection;
    }

    /**
     * Check if connected.
     */
    public function isConnected(): bool
    {
        return $this->connection->isConnected();
    }
}
