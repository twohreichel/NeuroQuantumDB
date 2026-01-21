<?php

declare(strict_types=1);

namespace NeuroQuantum\Exception;

/**
 * Exception for validation errors (HTTP 400).
 */
class ValidationException extends NeuroQuantumException
{
    /**
     * @param array<string, string[]> $errors Field-level validation errors
     */
    public function __construct(
        string $message = 'Validation failed',
        private readonly array $errors = [],
    ) {
        parent::__construct($message, 400, context: ['validation_errors' => $errors]);
    }

    /**
     * Get field-level validation errors.
     *
     * @return array<string, string[]>
     */
    public function getErrors(): array
    {
        return $this->errors;
    }

    /**
     * Create from API error response.
     *
     * @param array<string, mixed> $errorData
     */
    public static function fromApiResponse(array $errorData): self
    {
        $message = $errorData['message'] ?? 'Validation failed';
        $errors = $errorData['errors'] ?? [];

        return new self($message, $errors);
    }
}
