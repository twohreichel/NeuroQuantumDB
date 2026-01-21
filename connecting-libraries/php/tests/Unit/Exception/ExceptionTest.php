<?php

declare(strict_types=1);

namespace NeuroQuantum\Tests\Unit\Exception;

use NeuroQuantum\Exception\AuthenticationException;
use NeuroQuantum\Exception\AuthorizationException;
use NeuroQuantum\Exception\ConfigurationException;
use NeuroQuantum\Exception\ConflictException;
use NeuroQuantum\Exception\ConnectionException;
use NeuroQuantum\Exception\NeuroQuantumException;
use NeuroQuantum\Exception\NotFoundException;
use NeuroQuantum\Exception\QueryException;
use NeuroQuantum\Exception\RateLimitException;
use NeuroQuantum\Exception\ServerException;
use NeuroQuantum\Exception\ValidationException;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

final class ExceptionTest extends TestCase
{
    #[Test]
    public function base_exception_stores_context(): void
    {
        $exception = new NeuroQuantumException('Test error', 500, null, ['key' => 'value']);

        $this->assertSame('Test error', $exception->getMessage());
        $this->assertSame(500, $exception->getCode());
        $this->assertSame(['key' => 'value'], $exception->getContext());
    }

    #[Test]
    public function configuration_exception_for_missing_required(): void
    {
        $exception = ConfigurationException::missingRequired('API_KEY');

        $this->assertStringContainsString('API_KEY', $exception->getMessage());
        $this->assertSame('API_KEY', $exception->getContext()['missing_key']);
    }

    #[Test]
    public function connection_exception_for_failed_connection(): void
    {
        $exception = ConnectionException::failed('localhost', 8080, 'Connection refused');

        $this->assertStringContainsString('localhost:8080', $exception->getMessage());
        $this->assertStringContainsString('Connection refused', $exception->getMessage());
    }

    #[Test]
    public function connection_exception_for_timeout(): void
    {
        $exception = ConnectionException::timeout('localhost', 8080, 30);

        $this->assertStringContainsString('timeout', $exception->getMessage());
        $this->assertSame(30, $exception->getContext()['timeout']);
    }

    #[Test]
    public function authentication_exception_for_invalid_api_key(): void
    {
        $exception = AuthenticationException::invalidApiKey();

        $this->assertSame(401, $exception->getCode());
        $this->assertStringContainsString('API key', $exception->getMessage());
    }

    #[Test]
    public function authorization_exception_for_insufficient_permissions(): void
    {
        $exception = AuthorizationException::insufficientPermissions('admin');

        $this->assertSame(403, $exception->getCode());
        $this->assertStringContainsString('admin', $exception->getMessage());
    }

    #[Test]
    public function validation_exception_stores_field_errors(): void
    {
        $errors = ['email' => ['Invalid email format']];
        $exception = new ValidationException('Validation failed', $errors);

        $this->assertSame(400, $exception->getCode());
        $this->assertSame($errors, $exception->getErrors());
    }

    #[Test]
    public function query_exception_stores_query_info(): void
    {
        $exception = QueryException::syntaxError('Unexpected token', 'SELECT * FORM users', 10);

        $this->assertStringContainsString('Syntax error', $exception->getMessage());
        $this->assertSame('SELECT * FORM users', $exception->getQuery());
        $this->assertSame(10, $exception->getPosition());
    }

    #[Test]
    public function not_found_exception_for_table(): void
    {
        $exception = NotFoundException::table('missing_table');

        $this->assertSame(404, $exception->getCode());
        $this->assertStringContainsString('missing_table', $exception->getMessage());
    }

    #[Test]
    public function conflict_exception_for_duplicate_key(): void
    {
        $exception = ConflictException::duplicateKey('users', 'email');

        $this->assertSame(409, $exception->getCode());
        $this->assertStringContainsString('users', $exception->getMessage());
        $this->assertStringContainsString('email', $exception->getMessage());
    }

    #[Test]
    public function rate_limit_exception_stores_limits(): void
    {
        $exception = new RateLimitException('Rate limit exceeded', 60, 1000, 0);

        $this->assertSame(429, $exception->getCode());
        $this->assertSame(60, $exception->getRetryAfter());
        $this->assertSame(1000, $exception->getLimit());
        $this->assertSame(0, $exception->getRemaining());
    }

    #[Test]
    public function server_exception_from_status_code(): void
    {
        $exception500 = ServerException::fromStatusCode(500);
        $exception503 = ServerException::fromStatusCode(503);

        $this->assertSame(500, $exception500->getCode());
        $this->assertStringContainsString('Internal server error', $exception500->getMessage());

        $this->assertSame(503, $exception503->getCode());
        $this->assertStringContainsString('Service unavailable', $exception503->getMessage());
    }
}
