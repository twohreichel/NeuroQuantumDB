<?php

declare(strict_types=1);

namespace NeuroQuantum\Query;

use NeuroQuantum\Contract\QueryBuilderInterface;

/**
 * Fluent UPDATE query builder.
 */
final class UpdateBuilder implements QueryBuilderInterface
{
    use WhereClauseTrait;

    /** @var array<string, mixed> */
    private array $updates = [];

    public function __construct(
        private readonly string $table,
    ) {
    }

    /**
     * Create a new UPDATE builder.
     */
    public static function table(string $table): self
    {
        return new self($table);
    }

    /**
     * Set column value to update.
     */
    public function set(string $column, mixed $value): self
    {
        $this->updates[$column] = $value;
        return $this;
    }

    /**
     * Set multiple columns to update.
     *
     * @param array<string, mixed> $values
     */
    public function setMany(array $values): self
    {
        $this->updates = array_merge($this->updates, $values);
        return $this;
    }

    /**
     * Increment a column value.
     */
    public function increment(string $column, int|float $amount = 1): self
    {
        $this->updates[$column] = ['$increment' => $amount];
        return $this;
    }

    /**
     * Decrement a column value.
     */
    public function decrement(string $column, int|float $amount = 1): self
    {
        $this->updates[$column] = ['$decrement' => $amount];
        return $this;
    }

    public function toArray(): array
    {
        return [
            'table_name' => $this->table,
            'updates' => $this->normalizeUpdates(),
            'filters' => $this->buildFilters(),
        ];
    }

    public function toSql(): string
    {
        if ($this->updates === []) {
            return '';
        }

        $setClauses = [];
        foreach ($this->updates as $column => $value) {
            if (is_array($value) && isset($value['$increment'])) {
                $setClauses[] = sprintf(
                    '"%s" = "%s" + %s',
                    $column,
                    $column,
                    $value['$increment']
                );
            } elseif (is_array($value) && isset($value['$decrement'])) {
                $setClauses[] = sprintf(
                    '"%s" = "%s" - %s',
                    $column,
                    $column,
                    $value['$decrement']
                );
            } else {
                $setClauses[] = sprintf('"%s" = %s', $column, $this->formatValue($value));
            }
        }

        $sql = sprintf(
            'UPDATE "%s" SET %s',
            $this->table,
            implode(', ', $setClauses)
        );

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

    /**
     * Normalize updates for API.
     *
     * @return array<string, mixed>
     */
    private function normalizeUpdates(): array
    {
        $normalized = [];
        foreach ($this->updates as $column => $value) {
            if (is_array($value) && isset($value['$increment'])) {
                $normalized[$column] = ['operation' => 'increment', 'value' => $value['$increment']];
            } elseif (is_array($value) && isset($value['$decrement'])) {
                $normalized[$column] = ['operation' => 'decrement', 'value' => $value['$decrement']];
            } else {
                $normalized[$column] = $value;
            }
        }
        return $normalized;
    }
}
