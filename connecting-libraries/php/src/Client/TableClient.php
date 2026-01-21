<?php

declare(strict_types=1);

namespace NeuroQuantum\Client;

use NeuroQuantum\Query\DeleteBuilder;
use NeuroQuantum\Query\FilterOperator;
use NeuroQuantum\Query\InsertBuilder;
use NeuroQuantum\Query\SelectBuilder;
use NeuroQuantum\Query\SortDirection;
use NeuroQuantum\Query\UpdateBuilder;

/**
 * Client for table CRUD operations via REST endpoints.
 */
final class TableClient extends AbstractClient
{
    /**
     * Create a new table.
     *
     * @param TableSchema $schema Table schema definition
     * @param bool $ifNotExists Don't error if table exists
     * @return bool True if created successfully
     */
    public function create(TableSchema $schema, bool $ifNotExists = false): bool
    {
        $response = $this->connection->post('/tables', [
            'schema' => $schema->toArray(),
            'if_not_exists' => $ifNotExists,
        ]);

        return isset($response['created']) ? (bool) $response['created'] : true;
    }

    /**
     * Drop a table.
     *
     * @param string $table Table name
     * @param bool $ifExists Don't error if table doesn't exist
     * @return bool True if dropped successfully
     */
    public function drop(string $table, bool $ifExists = false): bool
    {
        $suffix = $ifExists ? ' IF EXISTS' : '';
        $queryClient = new QueryClient($this->connection);
        $result = $queryClient->execute(sprintf('DROP TABLE%s "%s"', $suffix, $table));
        return $result->success;
    }

    /**
     * Truncate a table.
     *
     * @param string $table Table name
     * @return bool True if truncated successfully
     */
    public function truncate(string $table): bool
    {
        $queryClient = new QueryClient($this->connection);
        $result = $queryClient->execute(sprintf('TRUNCATE TABLE "%s"', $table));
        return $result->success;
    }

    /**
     * Insert records into a table.
     *
     * @param string $table Table name
     * @param array<int, array<string, mixed>> $records Records to insert
     * @return InsertResult Insert result with counts and IDs
     */
    public function insert(string $table, array $records): InsertResult
    {
        $response = $this->connection->post('/tables/insert', [
            'table_name' => $table,
            'records' => $records,
        ]);

        return InsertResult::fromArray($response);
    }

    /**
     * Insert using a builder.
     */
    public function insertBuilder(InsertBuilder $builder): InsertResult
    {
        $response = $this->connection->post('/tables/insert', $builder->toArray());
        return InsertResult::fromArray($response);
    }

    /**
     * Query records from a table.
     *
     * @param string $table Table name
     * @param array<string, array{operator: string, value: mixed}> $filters Filters
     * @param array<int, array{column: string, direction: string}>|null $sort Sort order
     * @param int|null $limit Maximum rows
     * @param int $offset Offset
     * @param array<int, string>|null $columns Columns to select
     * @return array<int, array<string, mixed>> Query results
     */
    public function query(
        string $table,
        array $filters = [],
        ?array $sort = null,
        ?int $limit = null,
        int $offset = 0,
        ?array $columns = null,
    ): array {
        $request = ['table_name' => $table];

        if ($filters !== []) {
            $request['filters'] = $filters;
        }

        if ($sort !== null) {
            $request['sort'] = $sort;
        }

        if ($limit !== null) {
            $request['limit'] = $limit;
        }

        if ($offset > 0) {
            $request['offset'] = $offset;
        }

        if ($columns !== null) {
            $request['columns'] = $columns;
        }

        $response = $this->connection->post('/tables/query', $request);
        return $response['rows'] ?? [];
    }

    /**
     * Query using a SelectBuilder.
     *
     * @return array<int, array<string, mixed>>
     */
    public function select(SelectBuilder $builder): array
    {
        $response = $this->connection->post('/tables/query', $builder->toArray());
        return $response['rows'] ?? [];
    }

    /**
     * Update records in a table.
     *
     * @param string $table Table name
     * @param array<string, mixed> $updates Column updates
     * @param array<string, array{operator: string, value: mixed}> $filters Where conditions
     * @return int Number of rows updated
     */
    public function update(string $table, array $updates, array $filters = []): int
    {
        $response = $this->connection->put('/tables/update', [
            'table_name' => $table,
            'updates' => $updates,
            'filters' => $filters,
        ]);

        return (int) ($response['updated_count'] ?? 0);
    }

    /**
     * Update using an UpdateBuilder.
     *
     * @return int Number of rows updated
     */
    public function updateBuilder(UpdateBuilder $builder): int
    {
        $response = $this->connection->put('/tables/update', $builder->toArray());
        return (int) ($response['updated_count'] ?? 0);
    }

    /**
     * Delete records from a table.
     *
     * @param string $table Table name
     * @param array<string, array{operator: string, value: mixed}> $filters Where conditions
     * @return int Number of rows deleted
     */
    public function delete(string $table, array $filters = []): int
    {
        $response = $this->connection->delete('/tables/delete', [
            'table_name' => $table,
            'filters' => $filters,
        ]);

        return (int) ($response['deleted_count'] ?? 0);
    }

    /**
     * Delete using a DeleteBuilder.
     *
     * @return int Number of rows deleted
     */
    public function deleteBuilder(DeleteBuilder $builder): int
    {
        $response = $this->connection->delete('/tables/delete', $builder->toArray());
        return (int) ($response['deleted_count'] ?? 0);
    }

    /**
     * Find a single record by ID.
     *
     * @param string $table Table name
     * @param string|int $id Record ID
     * @param string $idColumn ID column name (default: 'id')
     * @return array<string, mixed>|null Record or null if not found
     */
    public function find(string $table, string|int $id, string $idColumn = 'id'): ?array
    {
        $rows = $this->query($table, [
            $idColumn => ['operator' => FilterOperator::Equals->value, 'value' => $id],
        ], null, 1);

        return $rows[0] ?? null;
    }

    /**
     * Check if a record exists.
     *
     * @param string $table Table name
     * @param array<string, array{operator: string, value: mixed}> $filters Conditions
     */
    public function exists(string $table, array $filters): bool
    {
        $rows = $this->query($table, $filters, null, 1, 0, ['1']);
        return $rows !== [];
    }

    /**
     * Count records in a table.
     *
     * @param string $table Table name
     * @param array<string, array{operator: string, value: mixed}> $filters Conditions
     */
    public function count(string $table, array $filters = []): int
    {
        $queryClient = new QueryClient($this->connection);
        $sql = sprintf('SELECT COUNT(*) as count FROM "%s"', $table);

        if ($filters !== []) {
            $builder = SelectBuilder::from($table)->select(['COUNT(*) as count']);
            foreach ($filters as $column => $filter) {
                $operator = FilterOperator::from($filter['operator']);
                $builder->where($column, $operator, $filter['value']);
            }
            $sql = $builder->toSql();
        }

        $result = $queryClient->execute($sql);
        return (int) ($result->value('count') ?? 0);
    }
}
