<?php

declare(strict_types=1);

namespace NeuroQuantum\Exception;

/**
 * Exception for SQL query errors.
 */
class QueryException extends NeuroQuantumException
{
    public function __construct(
        string $message,
        private readonly string $query = '',
        private readonly ?int $position = null,
    ) {
        parent::__construct($message, 400, context: [
            'query' => $query,
            'position' => $position,
        ]);
    }

    /**
     * Get the failed query.
     */
    public function getQuery(): string
    {
        return $this->query;
    }

    /**
     * Get error position in query.
     */
    public function getPosition(): ?int
    {
        return $this->position;
    }

    /**
     * Create for syntax error.
     */
    public static function syntaxError(string $message, string $query, ?int $position = null): self
    {
        return new self('Syntax error: ' . $message, $query, $position);
    }

    /**
     * Create for execution error.
     */
    public static function executionError(string $message, string $query): self
    {
        return new self('Execution error: ' . $message, $query);
    }
}
