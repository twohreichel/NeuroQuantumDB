<?php

declare(strict_types=1);

namespace NeuroQuantum\Exception;

/**
 * Exception for connection errors.
 */
class ConnectionException extends NeuroQuantumException
{
    /**
     * Create exception for connection failure.
     */
    public static function failed(string $host, int $port, string $reason = ''): self
    {
        $message = sprintf('Failed to connect to %s:%d', $host, $port);
        if ($reason !== '') {
            $message .= ': ' . $reason;
        }

        return new self($message, context: ['host' => $host, 'port' => $port]);
    }

    /**
     * Create exception for timeout.
     */
    public static function timeout(string $host, int $port, int $timeout): self
    {
        return new self(
            sprintf('Connection timeout after %d seconds to %s:%d', $timeout, $host, $port),
            context: ['host' => $host, 'port' => $port, 'timeout' => $timeout]
        );
    }
}
