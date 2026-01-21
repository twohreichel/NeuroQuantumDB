<?php

declare(strict_types=1);

namespace NeuroQuantum\Query;

use NeuroQuantum\Contract\QueryBuilderInterface;

/**
 * Fluent SELECT query builder.
 */
final class SelectBuilder implements QueryBuilderInterface
{
    use WhereClauseTrait;

    /** @var array<int, string> */
    private array $columns = ['*'];

    /** @var array<int, array{column: string, direction: SortDirection}> */
    private array $orderBy = [];

    private ?int $limit = null;
    private int $offset = 0;

    /** @var array<int, string> */
    private array $groupBy = [];

    /** @var array<int, array{column: string, operator: FilterOperator, value: mixed}> */
    private array $having = [];

    public function __construct(
        private readonly string $table,
    ) {
    }

    /**
     * Create a new SELECT builder.
     */
    public static function from(string $table): self
    {
        return new self($table);
    }

    /**
     * Set columns to select.
     *
     * @param string|array<int, string> $columns
     */
    public function select(string|array $columns = ['*']): self
    {
        $this->columns = is_array($columns) ? $columns : [$columns];
        return $this;
    }

    /**
     * Add ORDER BY clause.
     */
    public function orderBy(string $column, SortDirection|string $direction = SortDirection::Asc): self
    {
        if (is_string($direction)) {
            $direction = strtoupper($direction) === 'DESC' ? SortDirection::Desc : SortDirection::Asc;
        }

        $this->orderBy[] = ['column' => $column, 'direction' => $direction];
        return $this;
    }

    /**
     * Add ORDER BY DESC clause.
     */
    public function orderByDesc(string $column): self
    {
        return $this->orderBy($column, SortDirection::Desc);
    }

    /**
     * Set LIMIT clause.
     */
    public function limit(int $limit): self
    {
        $this->limit = $limit;
        return $this;
    }

    /**
     * Set OFFSET clause.
     */
    public function offset(int $offset): self
    {
        $this->offset = $offset;
        return $this;
    }

    /**
     * Convenience method for pagination.
     */
    public function paginate(int $page, int $perPage = 15): self
    {
        $this->limit = $perPage;
        $this->offset = ($page - 1) * $perPage;
        return $this;
    }

    /**
     * Add GROUP BY clause.
     *
     * @param string|array<int, string> $columns
     */
    public function groupBy(string|array $columns): self
    {
        $this->groupBy = is_array($columns) ? $columns : [$columns];
        return $this;
    }

    /**
     * Add HAVING clause.
     */
    public function having(string $column, FilterOperator|string $operator, mixed $value = null): self
    {
        if ($value === null && is_string($operator)) {
            $value = $operator;
            $operator = FilterOperator::Equals;
        }

        if (is_string($operator)) {
            $operator = match (strtolower($operator)) {
                '=' => FilterOperator::Equals,
                '!=' => FilterOperator::NotEquals,
                '>' => FilterOperator::GreaterThan,
                '<' => FilterOperator::LessThan,
                '>=' => FilterOperator::GreaterThanOrEquals,
                '<=' => FilterOperator::LessThanOrEquals,
                default => FilterOperator::Equals,
            };
        }

        $this->having[] = [
            'column' => $column,
            'operator' => $operator,
            'value' => $value,
        ];

        return $this;
    }

    public function toArray(): array
    {
        $query = [
            'table_name' => $this->table,
            'columns' => $this->columns === ['*'] ? null : $this->columns,
        ];

        $filters = $this->buildFilters();
        if ($filters !== []) {
            $query['filters'] = $filters;
        }

        if ($this->orderBy !== []) {
            $query['sort'] = array_map(
                fn($sort) => ['column' => $sort['column'], 'direction' => $sort['direction']->value],
                $this->orderBy
            );
        }

        if ($this->limit !== null) {
            $query['limit'] = $this->limit;
        }

        if ($this->offset > 0) {
            $query['offset'] = $this->offset;
        }

        return $query;
    }

    public function toSql(): string
    {
        $sql = 'SELECT ' . $this->buildColumnList();
        $sql .= ' FROM ' . $this->quoteIdentifier($this->table);

        $where = $this->buildWhereSql();
        if ($where !== '') {
            $sql .= ' ' . $where;
        }

        if ($this->groupBy !== []) {
            $sql .= ' GROUP BY ' . implode(', ', array_map(fn($c) => $this->quoteIdentifier($c), $this->groupBy));
        }

        if ($this->having !== []) {
            $sql .= ' ' . $this->buildHavingSql();
        }

        if ($this->orderBy !== []) {
            $sql .= ' ORDER BY ' . $this->buildOrderBySql();
        }

        if ($this->limit !== null) {
            $sql .= ' LIMIT ' . $this->limit;
        }

        if ($this->offset > 0) {
            $sql .= ' OFFSET ' . $this->offset;
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

    private function buildColumnList(): string
    {
        if ($this->columns === ['*']) {
            return '*';
        }

        return implode(', ', array_map(fn($c) => $this->quoteIdentifier($c), $this->columns));
    }

    private function buildOrderBySql(): string
    {
        return implode(', ', array_map(
            fn($sort) => $this->quoteIdentifier($sort['column']) . ' ' . $sort['direction']->toSql(),
            $this->orderBy
        ));
    }

    private function buildHavingSql(): string
    {
        if ($this->having === []) {
            return '';
        }

        $clauses = array_map(
            fn($h) => sprintf(
                '%s %s %s',
                $this->quoteIdentifier($h['column']),
                $h['operator']->toSql(),
                $this->formatValue($h['value'])
            ),
            $this->having
        );

        return 'HAVING ' . implode(' AND ', $clauses);
    }
}
