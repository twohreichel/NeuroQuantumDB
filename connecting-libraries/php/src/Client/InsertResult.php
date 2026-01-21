<?php

declare(strict_types=1);

namespace NeuroQuantum\Client;

/**
 * Value object for insert operation result.
 */
final readonly class InsertResult
{
    /**
     * @param array<int, string> $insertedIds IDs of inserted records
     * @param array<int, string>|null $errors Error messages if any
     */
    public function __construct(
        public int $insertedCount,
        public int $failedCount,
        public array $insertedIds,
        public ?array $errors,
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
            insertedCount: (int) ($data['inserted_count'] ?? 0),
            failedCount: (int) ($data['failed_count'] ?? 0),
            insertedIds: $data['inserted_ids'] ?? [],
            errors: $data['errors'] ?? null,
        );
    }

    /**
     * Check if all records were inserted successfully.
     */
    public function isSuccess(): bool
    {
        return $this->failedCount === 0;
    }

    /**
     * Get first inserted ID.
     */
    public function getLastInsertId(): ?string
    {
        return $this->insertedIds[array_key_last($this->insertedIds) ?? 0] ?? null;
    }
}
