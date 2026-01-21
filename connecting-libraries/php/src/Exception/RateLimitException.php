<?php

declare(strict_types=1);

namespace NeuroQuantum\Exception;

/**
 * Exception for rate limit exceeded (HTTP 429).
 */
class RateLimitException extends NeuroQuantumException
{
    public function __construct(
        string $message = 'Rate limit exceeded',
        private readonly ?int $retryAfter = null,
        private readonly ?int $limit = null,
        private readonly ?int $remaining = null,
    ) {
        parent::__construct($message, 429, context: [
            'retry_after' => $retryAfter,
            'limit' => $limit,
            'remaining' => $remaining,
        ]);
    }

    /**
     * Get seconds until rate limit resets.
     */
    public function getRetryAfter(): ?int
    {
        return $this->retryAfter;
    }

    /**
     * Get rate limit value.
     */
    public function getLimit(): ?int
    {
        return $this->limit;
    }

    /**
     * Get remaining requests.
     */
    public function getRemaining(): ?int
    {
        return $this->remaining;
    }
}
