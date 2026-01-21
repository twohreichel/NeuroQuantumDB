<?php

declare(strict_types=1);

namespace NeuroQuantum\Http;

use DateTimeImmutable;

/**
 * Value object representing an API response.
 */
final readonly class ApiResponse
{
    /**
     * @param array<string, mixed>|null $data Response data
     * @param array<string, mixed>|null $error Error details
     * @param array<string, mixed> $metadata Response metadata
     */
    public function __construct(
        public bool $success,
        public ?array $data,
        public ?array $error,
        public array $metadata,
        public int $httpStatusCode,
    ) {
    }

    /**
     * Parse API response from JSON array.
     *
     * @param array<string, mixed> $response Raw response data
     */
    public static function fromArray(array $response, int $httpStatusCode): self
    {
        return new self(
            success: (bool) ($response['success'] ?? false),
            data: $response['data'] ?? null,
            error: $response['error'] ?? null,
            metadata: $response['metadata'] ?? [],
            httpStatusCode: $httpStatusCode,
        );
    }

    /**
     * Check if the response was successful.
     */
    public function isSuccess(): bool
    {
        return $this->success && $this->httpStatusCode >= 200 && $this->httpStatusCode < 300;
    }

    /**
     * Get the response time in milliseconds.
     */
    public function getResponseTime(): ?float
    {
        return $this->metadata['response_time_ms'] ?? null;
    }

    /**
     * Get the request ID.
     */
    public function getRequestId(): ?string
    {
        return $this->metadata['request_id'] ?? null;
    }

    /**
     * Get the API version.
     */
    public function getVersion(): ?string
    {
        return $this->metadata['version'] ?? null;
    }

    /**
     * Get the timestamp.
     */
    public function getTimestamp(): ?DateTimeImmutable
    {
        $timestamp = $this->metadata['timestamp'] ?? null;
        if ($timestamp === null) {
            return null;
        }

        $dt = DateTimeImmutable::createFromFormat(DATE_ATOM, $timestamp);
        return $dt !== false ? $dt : null;
    }

    /**
     * Get error message.
     */
    public function getErrorMessage(): ?string
    {
        if ($this->error === null) {
            return null;
        }

        // Error can be a string or an object
        if (is_string($this->error)) {
            return $this->error;
        }

        // Handle object format like {"Unauthorized": "message"}
        $values = array_values($this->error);
        $firstValue = $values[0] ?? null;
        if (is_string($firstValue)) {
            return $firstValue;
        }

        return $this->error['message'] ?? json_encode($this->error);
    }

    /**
     * Get error type/code.
     */
    public function getErrorType(): ?string
    {
        if ($this->error === null || !is_array($this->error)) {
            return null;
        }

        // Handle object format like {"Unauthorized": "message"}
        $keys = array_keys($this->error);
        $firstKey = reset($keys);

        return is_string($firstKey) && !in_array($firstKey, ['message', 'code', 'errors'])
            ? $firstKey
            : null;
    }
}
