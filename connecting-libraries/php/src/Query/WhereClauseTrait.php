<?php

declare(strict_types=1);

namespace NeuroQuantum\Query;

/**
 * Trait for WHERE clause building.
 */
trait WhereClauseTrait
{
    /** @var array<int, array{column: string, operator: FilterOperator, value: mixed, boolean: string}> */
    protected array $wheres = [];

    /** @var array<string, mixed> */
    protected array $bindings = [];

    private int $bindingCounter = 0;

    /**
     * Add a WHERE clause.
     *
     * Supports two forms:
     * - where('column', 'operator', $value) - explicit operator
     * - where('column', $value) - shorthand for equals
     */
    public function where(string $column, FilterOperator|string|int|float|bool|null $operator, mixed $value = null): static
    {
        // Handle two-argument form: where('column', 'value')
        if ($value === null && !($operator instanceof FilterOperator) && !$this->isOperatorString($operator)) {
            $value = $operator;
            $operator = FilterOperator::Equals;
        }

        if (is_string($operator)) {
            $operator = $this->parseOperator($operator);
        }

        assert($operator instanceof FilterOperator);

        $this->wheres[] = [
            'column' => $column,
            'operator' => $operator,
            'value' => $value,
            'boolean' => 'AND',
        ];

        return $this;
    }

    /**
     * Add an OR WHERE clause.
     */
    public function orWhere(string $column, FilterOperator|string|int|float|bool|null $operator, mixed $value = null): static
    {
        if ($value === null && !($operator instanceof FilterOperator) && !$this->isOperatorString($operator)) {
            $value = $operator;
            $operator = FilterOperator::Equals;
        }

        if (is_string($operator)) {
            $operator = $this->parseOperator($operator);
        }

        assert($operator instanceof FilterOperator);

        $this->wheres[] = [
            'column' => $column,
            'operator' => $operator,
            'value' => $value,
            'boolean' => 'OR',
        ];

        return $this;
    }

    /**
     * Add a WHERE IN clause.
     *
     * @param array<int, mixed> $values
     */
    public function whereIn(string $column, array $values): static
    {
        return $this->where($column, FilterOperator::In, $values);
    }

    /**
     * Add a WHERE NOT IN clause.
     *
     * @param array<int, mixed> $values
     */
    public function whereNotIn(string $column, array $values): static
    {
        return $this->where($column, FilterOperator::NotIn, $values);
    }

    /**
     * Add a WHERE NULL clause.
     */
    public function whereNull(string $column): static
    {
        return $this->where($column, FilterOperator::IsNull, null);
    }

    /**
     * Add a WHERE NOT NULL clause.
     */
    public function whereNotNull(string $column): static
    {
        return $this->where($column, FilterOperator::IsNotNull, null);
    }

    /**
     * Add a WHERE LIKE clause.
     */
    public function whereLike(string $column, string $pattern): static
    {
        return $this->where($column, FilterOperator::Like, $pattern);
    }

    /**
     * Add a WHERE column contains value clause.
     */
    public function whereContains(string $column, string $value): static
    {
        return $this->where($column, FilterOperator::Contains, $value);
    }

    /**
     * Add a WHERE column starts with value clause.
     */
    public function whereStartsWith(string $column, string $value): static
    {
        return $this->where($column, FilterOperator::StartsWith, $value);
    }

    /**
     * Add a WHERE column ends with value clause.
     */
    public function whereEndsWith(string $column, string $value): static
    {
        return $this->where($column, FilterOperator::EndsWith, $value);
    }

    /**
     * Build filters array for API request.
     *
     * @return array<string, array{operator: string, value: mixed}>
     */
    protected function buildFilters(): array
    {
        $filters = [];
        foreach ($this->wheres as $where) {
            $filters[$where['column']] = [
                'operator' => $where['operator']->value,
                'value' => $where['value'],
            ];
        }
        return $filters;
    }

    /**
     * Build WHERE SQL clause.
     */
    protected function buildWhereSql(): string
    {
        if ($this->wheres === []) {
            return '';
        }

        $clauses = [];
        foreach ($this->wheres as $index => $where) {
            $clause = $this->buildWhereCondition($where);

            if ($index === 0) {
                $clauses[] = $clause;
            } else {
                $clauses[] = $where['boolean'] . ' ' . $clause;
            }
        }

        return 'WHERE ' . implode(' ', $clauses);
    }

    /**
     * Build a single WHERE condition.
     *
     * @param array{column: string, operator: FilterOperator, value: mixed, boolean: string} $where
     */
    private function buildWhereCondition(array $where): string
    {
        $column = $this->quoteIdentifier($where['column']);
        $operator = $where['operator'];

        return match ($operator) {
            FilterOperator::IsNull, FilterOperator::IsNotNull => sprintf(
                '%s %s',
                $column,
                $operator->toSql()
            ),
            FilterOperator::In, FilterOperator::NotIn => sprintf(
                '%s %s (%s)',
                $column,
                $operator->toSql(),
                $this->formatValueList($where['value'])
            ),
            FilterOperator::Contains => sprintf(
                '%s LIKE %s',
                $column,
                $this->bindValue('%' . $where['value'] . '%')
            ),
            FilterOperator::StartsWith => sprintf(
                '%s LIKE %s',
                $column,
                $this->bindValue($where['value'] . '%')
            ),
            FilterOperator::EndsWith => sprintf(
                '%s LIKE %s',
                $column,
                $this->bindValue('%' . $where['value'])
            ),
            default => sprintf(
                '%s %s %s',
                $column,
                $operator->toSql(),
                $this->bindValue($where['value'])
            ),
        };
    }

    /**
     * Bind a value and return placeholder.
     */
    protected function bindValue(mixed $value): string
    {
        $key = ':p' . $this->bindingCounter++;
        $this->bindings[$key] = $value;
        return $this->formatValue($value);
    }

    /**
     * Format value for SQL.
     */
    protected function formatValue(mixed $value): string
    {
        if ($value === null) {
            return 'NULL';
        }

        if (is_bool($value)) {
            return $value ? 'TRUE' : 'FALSE';
        }

        if (is_int($value) || is_float($value)) {
            return (string) $value;
        }

        // Escape string value
        return "'" . str_replace("'", "''", (string) $value) . "'";
    }

    /**
     * Format array of values for IN clause.
     *
     * @param array<int, mixed> $values
     */
    protected function formatValueList(array $values): string
    {
        return implode(', ', array_map(fn($v) => $this->formatValue($v), $values));
    }

    /**
     * Quote identifier.
     */
    protected function quoteIdentifier(string $identifier): string
    {
        // Simple quoting for column/table names
        if (str_contains($identifier, '.')) {
            $parts = explode('.', $identifier);
            return implode('.', array_map(fn($p) => '"' . $p . '"', $parts));
        }
        return '"' . $identifier . '"';
    }

    /**
     * Parse string operator to FilterOperator.
     */
    private function parseOperator(string $operator): FilterOperator
    {
        return match (strtolower($operator)) {
            '=', '==' => FilterOperator::Equals,
            '!=', '<>' => FilterOperator::NotEquals,
            '>' => FilterOperator::GreaterThan,
            '<' => FilterOperator::LessThan,
            '>=' => FilterOperator::GreaterThanOrEquals,
            '<=' => FilterOperator::LessThanOrEquals,
            'in' => FilterOperator::In,
            'not in' => FilterOperator::NotIn,
            'like' => FilterOperator::Like,
            'not like' => FilterOperator::NotLike,
            default => FilterOperator::Equals,
        };
    }

    /**
     * Check if a value is an operator string.
     */
    private function isOperatorString(mixed $value): bool
    {
        if (!is_string($value)) {
            return false;
        }

        return in_array(strtolower($value), [
            '=', '==', '!=', '<>', '>', '<', '>=', '<=',
            'in', 'not in', 'like', 'not like',
        ], true);
    }
}
