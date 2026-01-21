<?php

declare(strict_types=1);

namespace NeuroQuantum\Query;

use NeuroQuantum\Contract\QueryBuilderInterface;

/**
 * Fluent INSERT query builder.
 */
final class InsertBuilder implements QueryBuilderInterface
{
    /** @var array<int, array<string, mixed>> */
    private array $records = [];

    private int $batchSize = 1000;
    private string $onConflict = 'Error';

    /** @var array<string, mixed> */
    protected array $bindings = [];

    public function __construct(
        private readonly string $table,
    ) {
    }

    /**
     * Create a new INSERT builder.
     */
    public static function into(string $table): self
    {
        return new self($table);
    }

    /**
     * Add a single record to insert.
     *
     * @param array<string, mixed> $record
     */
    public function values(array $record): self
    {
        $this->records[] = $record;
        return $this;
    }

    /**
     * Add multiple records to insert.
     *
     * @param array<int, array<string, mixed>> $records
     */
    public function insert(array $records): self
    {
        $this->records = array_merge($this->records, $records);
        return $this;
    }

    /**
     * Set batch size for bulk inserts.
     */
    public function batchSize(int $size): self
    {
        $this->batchSize = $size;
        return $this;
    }

    /**
     * Set conflict handling strategy.
     *
     * @param string $strategy One of: 'Error', 'Ignore', 'Replace', 'Update'
     */
    public function onConflict(string $strategy): self
    {
        $this->onConflict = ucfirst(strtolower($strategy));
        return $this;
    }

    /**
     * Ignore conflicts.
     */
    public function orIgnore(): self
    {
        return $this->onConflict('Ignore');
    }

    /**
     * Replace on conflicts.
     */
    public function orReplace(): self
    {
        return $this->onConflict('Replace');
    }

    public function toArray(): array
    {
        return [
            'table_name' => $this->table,
            'records' => $this->records,
            'batch_size' => $this->batchSize,
            'on_conflict' => $this->onConflict,
        ];
    }

    public function toSql(): string
    {
        if ($this->records === []) {
            return '';
        }

        $columns = array_keys($this->records[0]);
        $columnList = implode(', ', array_map(fn($c) => '"' . $c . '"', $columns));

        $valueRows = [];
        foreach ($this->records as $record) {
            $values = array_map(fn($v) => $this->formatValue($v), array_values($record));
            $valueRows[] = '(' . implode(', ', $values) . ')';
        }

        $sql = sprintf(
            'INSERT INTO "%s" (%s) VALUES %s',
            $this->table,
            $columnList,
            implode(', ', $valueRows)
        );

        if ($this->onConflict !== 'Error') {
            $sql = 'INSERT OR ' . strtoupper($this->onConflict) . substr($sql, 6);
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
     * Get records.
     *
     * @return array<int, array<string, mixed>>
     */
    public function getRecords(): array
    {
        return $this->records;
    }

    /**
     * Format value for SQL.
     */
    private function formatValue(mixed $value): string
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

        if (is_array($value)) {
            return "'" . str_replace("'", "''", json_encode($value) ?: '[]') . "'";
        }

        return "'" . str_replace("'", "''", (string) $value) . "'";
    }
}
