<?php

declare(strict_types=1);

namespace NeuroQuantum\Http;

use NeuroQuantum\Exception\AuthenticationException;
use NeuroQuantum\Exception\AuthorizationException;
use NeuroQuantum\Exception\ConflictException;
use NeuroQuantum\Exception\NeuroQuantumException;
use NeuroQuantum\Exception\NotFoundException;
use NeuroQuantum\Exception\QueryException;
use NeuroQuantum\Exception\RateLimitException;
use NeuroQuantum\Exception\ServerException;
use NeuroQuantum\Exception\ValidationException;

/**
 * Factory for creating appropriate exceptions from API responses.
 */
final class ExceptionFactory
{
    /**
     * Create exception from API response.
     *
     * @param array<string, string|int|null> $headers Response headers
     */
    public static function fromResponse(ApiResponse $response, array $headers = []): NeuroQuantumException
    {
        $message = $response->getErrorMessage() ?? 'Unknown error';
        $errorType = $response->getErrorType();
        $statusCode = $response->httpStatusCode;

        return match ($statusCode) {
            400 => self::createValidationOrQueryException($response, $message, $errorType),
            401 => AuthenticationException::fromMessage($message),
            403 => AuthorizationException::fromMessage($message),
            404 => NotFoundException::fromMessage($message),
            409 => ConflictException::fromMessage($message),
            429 => self::createRateLimitException($message, $headers),
            default => $statusCode >= 500
                ? ServerException::fromStatusCode($statusCode, $message)
                : new NeuroQuantumException($message, $statusCode),
        };
    }

    /**
     * Create validation or query exception based on error type.
     */
    private static function createValidationOrQueryException(
        ApiResponse $response,
        string $message,
        ?string $errorType
    ): NeuroQuantumException {
        // Check if it's a SQL/query error
        if ($errorType !== null && str_contains(strtolower($errorType), 'query')) {
            return QueryException::executionError($message, '');
        }

        if ($errorType !== null && str_contains(strtolower($errorType), 'syntax')) {
            return QueryException::syntaxError($message, '');
        }

        // Regular validation error
        return ValidationException::fromApiResponse($response->error ?? ['message' => $message]);
    }

    /**
     * Create rate limit exception with headers.
     *
     * @param array<string, string|int|null> $headers
     */
    private static function createRateLimitException(string $message, array $headers): RateLimitException
    {
        $retryAfter = isset($headers['X-RateLimit-Reset'])
            ? (int) $headers['X-RateLimit-Reset'] - time()
            : null;
        $limit = isset($headers['X-RateLimit-Limit']) ? (int) $headers['X-RateLimit-Limit'] : null;
        $remaining = isset($headers['X-RateLimit-Remaining']) ? (int) $headers['X-RateLimit-Remaining'] : null;

        return new RateLimitException($message, $retryAfter, $limit, $remaining);
    }
}
