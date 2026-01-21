<?php

declare(strict_types=1);

namespace NeuroQuantum\Client;

/**
 * Value object representing query execution result.
 */
final readonly class QueryResult
{
    /**
     * @param array<int, array<string, mixed>> $rows Result rows
     * @param array<int, string> $columns Column names
     */
    public function __construct(
        public bool $success,
        public int $rowsAffected,
        public array $rows,
        public array $columns,
        public ?string $error,
        public ?float $executionTimeMs,
    ) {
    }

    /**
     * Create from API response array.
     *
     * @param array<string, mixed> $data
     */
    public static function fromArray(array $data): self
    {
        return new self(
            success: ($data['success'] ?? true) && !isset($data['error']),
            rowsAffected: (int) ($data['rows_affected'] ?? 0),
            rows: $data['rows'] ?? [],
            columns: $data['columns'] ?? [],
            error: $data['error'] ?? null,
            executionTimeMs: isset($data['execution_time_ms']) ? (float) $data['execution_time_ms'] : null,
        );
    }

    /**
     * Get first row or null.
     *
     * @return array<string, mixed>|null
     */
    public function first(): ?array
    {
        return $this->rows[0] ?? null;
    }

    /**
     * Get first column value from first row.
     */
    public function value(string $column): mixed
    {
        return $this->rows[0][$column] ?? null;
    }

    /**
     * Get single column values as array.
     *
     * @return array<int, mixed>
     */
    public function pluck(string $column): array
    {
        return array_column($this->rows, $column);
    }

    /**
     * Check if result has any rows.
     */
    public function isEmpty(): bool
    {
        return $this->rows === [];
    }

    /**
     * Get row count.
     */
    public function count(): int
    {
        return count($this->rows);
    }
}
