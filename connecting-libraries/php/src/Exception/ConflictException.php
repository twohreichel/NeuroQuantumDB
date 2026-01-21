<?php

declare(strict_types=1);

namespace NeuroQuantum\Exception;

/**
 * Exception for conflict errors (HTTP 409).
 */
class ConflictException extends NeuroQuantumException
{
    /**
     * Create for table already exists.
     */
    public static function tableExists(string $tableName): self
    {
        return new self(
            sprintf('Table already exists: %s', $tableName),
            409,
            context: ['table' => $tableName]
        );
    }

    /**
     * Create for duplicate key.
     */
    public static function duplicateKey(string $table, string $key): self
    {
        return new self(
            sprintf('Duplicate key in table "%s": %s', $table, $key),
            409,
            context: ['table' => $table, 'key' => $key]
        );
    }

    /**
     * Create from API error message.
     */
    public static function fromMessage(string $message): self
    {
        return new self($message, 409);
    }
}
