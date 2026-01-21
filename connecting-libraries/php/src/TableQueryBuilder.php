<?php

declare(strict_types=1);

namespace NeuroQuantum;

use NeuroQuantum\Client\InsertResult;
use NeuroQuantum\Client\TableClient;
use NeuroQuantum\Query\DeleteBuilder;
use NeuroQuantum\Query\FilterOperator;
use NeuroQuantum\Query\InsertBuilder;
use NeuroQuantum\Query\SelectBuilder;
use NeuroQuantum\Query\SortDirection;
use NeuroQuantum\Query\UpdateBuilder;

/**
 * Fluent query builder for table operations.
 *
 * Provides a Laravel-like interface for building queries.
 *
 * @example
 * ```php
 * $users = $db->table('users')
 *     ->select(['id', 'name'])
 *     ->where('age', '>', 21)
 *     ->orderBy('name')
 *     ->limit(10)
 *     ->get();
 * ```
 */
final class TableQueryBuilder
{
    private SelectBuilder $selectBuilder;
    private ?UpdateBuilder $updateBuilder = null;
    private ?DeleteBuilder $deleteBuilder = null;

    public function __construct(
        private readonly TableClient $client,
        private readonly string $table,
    ) {
        $this->selectBuilder = SelectBuilder::from($table);
    }

    // ========================================
    // SELECT Methods
    // ========================================

    /**
     * Set columns to select.
     *
     * @param string|array<int, string> $columns
     */
    public function select(string|array $columns = ['*']): self
    {
        $this->selectBuilder->select($columns);
        return $this;
    }

    /**
     * Add WHERE clause.
     */
    public function where(string $column, FilterOperator|string $operator, mixed $value = null): self
    {
        $this->selectBuilder->where($column, $operator, $value);
        return $this;
    }

    /**
     * Add OR WHERE clause.
     */
    public function orWhere(string $column, FilterOperator|string $operator, mixed $value = null): self
    {
        $this->selectBuilder->orWhere($column, $operator, $value);
        return $this;
    }

    /**
     * Add WHERE IN clause.
     *
     * @param array<int, mixed> $values
     */
    public function whereIn(string $column, array $values): self
    {
        $this->selectBuilder->whereIn($column, $values);
        return $this;
    }

    /**
     * Add WHERE NOT IN clause.
     *
     * @param array<int, mixed> $values
     */
    public function whereNotIn(string $column, array $values): self
    {
        $this->selectBuilder->whereNotIn($column, $values);
        return $this;
    }

    /**
     * Add WHERE NULL clause.
     */
    public function whereNull(string $column): self
    {
        $this->selectBuilder->whereNull($column);
        return $this;
    }

    /**
     * Add WHERE NOT NULL clause.
     */
    public function whereNotNull(string $column): self
    {
        $this->selectBuilder->whereNotNull($column);
        return $this;
    }

    /**
     * Add WHERE LIKE clause.
     */
    public function whereLike(string $column, string $pattern): self
    {
        $this->selectBuilder->whereLike($column, $pattern);
        return $this;
    }

    /**
     * Add ORDER BY clause.
     */
    public function orderBy(string $column, SortDirection|string $direction = SortDirection::Asc): self
    {
        $this->selectBuilder->orderBy($column, $direction);
        return $this;
    }

    /**
     * Add ORDER BY DESC clause.
     */
    public function orderByDesc(string $column): self
    {
        $this->selectBuilder->orderByDesc($column);
        return $this;
    }

    /**
     * Set LIMIT.
     */
    public function limit(int $limit): self
    {
        $this->selectBuilder->limit($limit);
        return $this;
    }

    /**
     * Set OFFSET.
     */
    public function offset(int $offset): self
    {
        $this->selectBuilder->offset($offset);
        return $this;
    }

    /**
     * Paginate results.
     */
    public function paginate(int $page, int $perPage = 15): self
    {
        $this->selectBuilder->paginate($page, $perPage);
        return $this;
    }

    /**
     * Add GROUP BY.
     *
     * @param string|array<int, string> $columns
     */
    public function groupBy(string|array $columns): self
    {
        $this->selectBuilder->groupBy($columns);
        return $this;
    }

    /**
     * Add HAVING clause.
     */
    public function having(string $column, FilterOperator|string $operator, mixed $value = null): self
    {
        $this->selectBuilder->having($column, $operator, $value);
        return $this;
    }

    /**
     * Execute query and get results.
     *
     * @return array<int, array<string, mixed>>
     */
    public function get(): array
    {
        return $this->client->select($this->selectBuilder);
    }

    /**
     * Execute query and get first result.
     *
     * @return array<string, mixed>|null
     */
    public function first(): ?array
    {
        $this->limit(1);
        $results = $this->get();
        return $results[0] ?? null;
    }

    /**
     * Get single column values as array.
     *
     * @return array<int, mixed>
     */
    public function pluck(string $column): array
    {
        $this->select([$column]);
        return array_column($this->get(), $column);
    }

    /**
     * Get single value.
     */
    public function value(string $column): mixed
    {
        $row = $this->first();
        return $row[$column] ?? null;
    }

    /**
     * Check if any records exist.
     */
    public function exists(): bool
    {
        return $this->first() !== null;
    }

    /**
     * Count records.
     */
    public function count(): int
    {
        return $this->client->count($this->table, $this->selectBuilder->toArray()['filters'] ?? []);
    }

    // ========================================
    // INSERT Methods
    // ========================================

    /**
     * Insert a single record.
     *
     * @param array<string, mixed> $values
     */
    public function insertOne(array $values): InsertResult
    {
        return $this->client->insert($this->table, [$values]);
    }

    /**
     * Insert multiple records.
     *
     * @param array<int, array<string, mixed>> $records
     */
    public function insertMany(array $records): InsertResult
    {
        return $this->client->insert($this->table, $records);
    }

    // ========================================
    // UPDATE Methods
    // ========================================

    /**
     * Update matching records.
     *
     * @param array<string, mixed> $values
     * @return int Number of rows updated
     */
    public function updateSet(array $values): int
    {
        $this->updateBuilder = UpdateBuilder::table($this->table)->setMany($values);

        // Copy where conditions from select builder
        foreach ($this->selectBuilder->toArray()['filters'] ?? [] as $column => $filter) {
            $operator = FilterOperator::from($filter['operator']);
            $this->updateBuilder->where($column, $operator, $filter['value']);
        }

        return $this->client->updateBuilder($this->updateBuilder);
    }

    /**
     * Increment a column value.
     *
     * @return int Number of rows updated
     */
    public function increment(string $column, int|float $amount = 1): int
    {
        $this->updateBuilder = UpdateBuilder::table($this->table)->increment($column, $amount);

        foreach ($this->selectBuilder->toArray()['filters'] ?? [] as $col => $filter) {
            $operator = FilterOperator::from($filter['operator']);
            $this->updateBuilder->where($col, $operator, $filter['value']);
        }

        return $this->client->updateBuilder($this->updateBuilder);
    }

    /**
     * Decrement a column value.
     *
     * @return int Number of rows updated
     */
    public function decrement(string $column, int|float $amount = 1): int
    {
        $this->updateBuilder = UpdateBuilder::table($this->table)->decrement($column, $amount);

        foreach ($this->selectBuilder->toArray()['filters'] ?? [] as $col => $filter) {
            $operator = FilterOperator::from($filter['operator']);
            $this->updateBuilder->where($col, $operator, $filter['value']);
        }

        return $this->client->updateBuilder($this->updateBuilder);
    }

    // ========================================
    // DELETE Methods
    // ========================================

    /**
     * Delete matching records.
     *
     * @return int Number of rows deleted
     */
    public function deleteRows(): int
    {
        $this->deleteBuilder = DeleteBuilder::from($this->table);

        foreach ($this->selectBuilder->toArray()['filters'] ?? [] as $column => $filter) {
            $operator = FilterOperator::from($filter['operator']);
            $this->deleteBuilder->where($column, $operator, $filter['value']);
        }

        return $this->client->deleteBuilder($this->deleteBuilder);
    }

    // ========================================
    // Utility Methods
    // ========================================

    /**
     * Get the underlying select builder.
     */
    public function getSelectBuilder(): SelectBuilder
    {
        return $this->selectBuilder;
    }

    /**
     * Get the generated SQL.
     */
    public function toSql(): string
    {
        return $this->selectBuilder->toSql();
    }

    /**
     * Get the query array for API.
     *
     * @return array<string, mixed>
     */
    public function toArray(): array
    {
        return $this->selectBuilder->toArray();
    }
}
