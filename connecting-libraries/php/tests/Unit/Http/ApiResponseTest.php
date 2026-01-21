<?php

declare(strict_types=1);

namespace NeuroQuantum\Tests\Unit\Http;

use NeuroQuantum\Http\ApiResponse;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

final class ApiResponseTest extends TestCase
{
    #[Test]
    public function it_creates_response_from_array(): void
    {
        $data = [
            'success' => true,
            'data' => ['id' => 1, 'name' => 'Test'],
            'error' => null,
            'metadata' => [
                'response_time_ms' => 12.5,
                'request_id' => 'abc-123',
                'version' => '0.1.0',
            ],
        ];

        $response = ApiResponse::fromArray($data, 200);

        $this->assertTrue($response->success);
        $this->assertSame(['id' => 1, 'name' => 'Test'], $response->data);
        $this->assertNull($response->error);
        $this->assertSame(200, $response->httpStatusCode);
    }

    #[Test]
    public function it_checks_success_status(): void
    {
        $successResponse = ApiResponse::fromArray(['success' => true], 200);
        $failureResponse = ApiResponse::fromArray(['success' => false], 400);

        $this->assertTrue($successResponse->isSuccess());
        $this->assertFalse($failureResponse->isSuccess());
    }

    #[Test]
    public function it_gets_response_time(): void
    {
        $response = ApiResponse::fromArray([
            'success' => true,
            'metadata' => ['response_time_ms' => 42.5],
        ], 200);

        $this->assertSame(42.5, $response->getResponseTime());
    }

    #[Test]
    public function it_gets_request_id(): void
    {
        $response = ApiResponse::fromArray([
            'success' => true,
            'metadata' => ['request_id' => 'xyz-789'],
        ], 200);

        $this->assertSame('xyz-789', $response->getRequestId());
    }

    #[Test]
    public function it_gets_version(): void
    {
        $response = ApiResponse::fromArray([
            'success' => true,
            'metadata' => ['version' => '1.2.3'],
        ], 200);

        $this->assertSame('1.2.3', $response->getVersion());
    }

    #[Test]
    public function it_parses_timestamp(): void
    {
        $response = ApiResponse::fromArray([
            'success' => true,
            'metadata' => ['timestamp' => '2026-01-21T12:00:00+00:00'],
        ], 200);

        $timestamp = $response->getTimestamp();

        $this->assertNotNull($timestamp);
        $this->assertSame('2026-01-21', $timestamp->format('Y-m-d'));
    }

    #[Test]
    public function it_returns_null_for_invalid_timestamp(): void
    {
        $response = ApiResponse::fromArray([
            'success' => true,
            'metadata' => ['timestamp' => 'invalid'],
        ], 200);

        $this->assertNull($response->getTimestamp());
    }

    #[Test]
    public function it_gets_error_message_from_string(): void
    {
        $response = ApiResponse::fromArray([
            'success' => false,
            'error' => ['Unauthorized' => 'Invalid API key'],
        ], 401);

        $this->assertSame('Invalid API key', $response->getErrorMessage());
    }

    #[Test]
    public function it_gets_error_type(): void
    {
        $response = ApiResponse::fromArray([
            'success' => false,
            'error' => ['Unauthorized' => 'Invalid API key'],
        ], 401);

        $this->assertSame('Unauthorized', $response->getErrorType());
    }

    #[Test]
    public function it_handles_missing_metadata(): void
    {
        $response = ApiResponse::fromArray(['success' => true], 200);

        $this->assertNull($response->getResponseTime());
        $this->assertNull($response->getRequestId());
        $this->assertNull($response->getVersion());
        $this->assertNull($response->getTimestamp());
    }

    #[Test]
    public function it_handles_missing_error(): void
    {
        $response = ApiResponse::fromArray(['success' => true], 200);

        $this->assertNull($response->getErrorMessage());
        $this->assertNull($response->getErrorType());
    }
}
