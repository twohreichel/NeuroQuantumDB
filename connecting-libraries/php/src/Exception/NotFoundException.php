<?php

declare(strict_types=1);

namespace NeuroQuantum\Exception;

/**
 * Exception for resource not found errors (HTTP 404).
 */
class NotFoundException extends NeuroQuantumException
{
    /**
     * Create for table not found.
     */
    public static function table(string $tableName): self
    {
        return new self(
            sprintf('Table not found: %s', $tableName),
            404,
            context: ['table' => $tableName]
        );
    }

    /**
     * Create for record not found.
     */
    public static function record(string $table, string|int $id): self
    {
        return new self(
            sprintf('Record not found in table "%s" with id: %s', $table, $id),
            404,
            context: ['table' => $table, 'id' => $id]
        );
    }

    /**
     * Create from API error message.
     */
    public static function fromMessage(string $message): self
    {
        return new self($message, 404);
    }
}
