<?php

declare(strict_types=1);

namespace NeuroQuantum\Query;

use NeuroQuantum\Contract\QueryBuilderInterface;

/**
 * Fluent DELETE query builder.
 */
final class DeleteBuilder implements QueryBuilderInterface
{
    use WhereClauseTrait;

    public function __construct(
        private readonly string $table,
    ) {
    }

    /**
     * Create a new DELETE builder.
     */
    public static function from(string $table): self
    {
        return new self($table);
    }

    public function toArray(): array
    {
        return [
            'table_name' => $this->table,
            'filters' => $this->buildFilters(),
        ];
    }

    public function toSql(): string
    {
        $sql = sprintf('DELETE FROM "%s"', $this->table);

        $where = $this->buildWhereSql();
        if ($where !== '') {
            $sql .= ' ' . $where;
        }

        return $sql;
    }

    public function getBindings(): array
    {
        return $this->bindings;
    }

    /**
     * Get the table name.
     */
    public function getTable(): string
    {
        return $this->table;
    }
}
