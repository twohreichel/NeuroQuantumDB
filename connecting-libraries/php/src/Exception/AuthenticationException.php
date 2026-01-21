<?php

declare(strict_types=1);

namespace NeuroQuantum\Exception;

/**
 * Exception for authentication errors (HTTP 401).
 */
class AuthenticationException extends NeuroQuantumException
{
    /**
     * Create exception for invalid API key.
     */
    public static function invalidApiKey(): self
    {
        return new self('Invalid or missing API key', 401);
    }

    /**
     * Create exception for expired API key.
     */
    public static function expiredApiKey(): self
    {
        return new self('API key has expired', 401);
    }

    /**
     * Create from API error message.
     */
    public static function fromMessage(string $message): self
    {
        return new self($message, 401);
    }
}
