<?php

declare(strict_types=1);

namespace NeuroQuantum\Exception;

/**
 * Exception for authorization/permission errors (HTTP 403).
 */
class AuthorizationException extends NeuroQuantumException
{
    /**
     * Create exception for insufficient permissions.
     */
    public static function insufficientPermissions(string $required): self
    {
        return new self(
            sprintf('Insufficient permissions. Required: %s', $required),
            403,
            context: ['required_permission' => $required]
        );
    }

    /**
     * Create from API error message.
     */
    public static function fromMessage(string $message): self
    {
        return new self($message, 403);
    }
}
