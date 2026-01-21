<?php

declare(strict_types=1);

namespace NeuroQuantum\Exception;

use Exception;
use Throwable;

/**
 * Base exception for all NeuroQuantumDB driver exceptions.
 */
class NeuroQuantumException extends Exception
{
    /**
     * @param array<string, mixed> $context Additional context data
     */
    public function __construct(
        string $message = '',
        int $code = 0,
        ?Throwable $previous = null,
        protected readonly array $context = [],
    ) {
        parent::__construct($message, $code, $previous);
    }

    /**
     * Get additional context data.
     *
     * @return array<string, mixed>
     */
    public function getContext(): array
    {
        return $this->context;
    }
}
