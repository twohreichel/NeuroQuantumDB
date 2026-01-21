<?php

declare(strict_types=1);

namespace NeuroQuantum\Exception;

/**
 * Exception for server errors (HTTP 5xx).
 */
class ServerException extends NeuroQuantumException
{
    /**
     * Create for internal server error.
     */
    public static function internalError(string $message = 'Internal server error'): self
    {
        return new self($message, 500);
    }

    /**
     * Create for service unavailable.
     */
    public static function unavailable(string $message = 'Service temporarily unavailable'): self
    {
        return new self($message, 503);
    }

    /**
     * Create from HTTP status code.
     */
    public static function fromStatusCode(int $statusCode, string $message = ''): self
    {
        $defaultMessage = match ($statusCode) {
            500 => 'Internal server error',
            501 => 'Not implemented',
            502 => 'Bad gateway',
            503 => 'Service unavailable',
            504 => 'Gateway timeout',
            default => 'Server error',
        };

        return new self($message !== '' ? $message : $defaultMessage, $statusCode);
    }
}
