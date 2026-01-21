<?php

declare(strict_types=1);

namespace NeuroQuantum\Tests\Unit\Client;

use NeuroQuantum\Client\InsertResult;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

final class InsertResultTest extends TestCase
{
    #[Test]
    public function it_creates_result_from_array(): void
    {
        $data = [
            'inserted_count' => 3,
            'failed_count' => 0,
            'inserted_ids' => ['1', '2', '3'],
            'errors' => null,
        ];

        $result = InsertResult::fromArray($data);

        $this->assertSame(3, $result->insertedCount);
        $this->assertSame(0, $result->failedCount);
        $this->assertSame(['1', '2', '3'], $result->insertedIds);
        $this->assertNull($result->errors);
    }

    #[Test]
    public function it_checks_success(): void
    {
        $successResult = InsertResult::fromArray(['inserted_count' => 2, 'failed_count' => 0]);
        $failedResult = InsertResult::fromArray(['inserted_count' => 1, 'failed_count' => 1]);

        $this->assertTrue($successResult->isSuccess());
        $this->assertFalse($failedResult->isSuccess());
    }

    #[Test]
    public function it_gets_last_insert_id(): void
    {
        $result = InsertResult::fromArray([
            'inserted_ids' => ['10', '11', '12'],
        ]);

        $this->assertSame('12', $result->getLastInsertId());
    }

    #[Test]
    public function it_returns_null_for_empty_ids(): void
    {
        $result = InsertResult::fromArray([
            'inserted_ids' => [],
        ]);

        $this->assertNull($result->getLastInsertId());
    }

    #[Test]
    public function it_handles_errors(): void
    {
        $result = InsertResult::fromArray([
            'inserted_count' => 1,
            'failed_count' => 2,
            'errors' => ['Duplicate key', 'Invalid value'],
        ]);

        $this->assertSame(['Duplicate key', 'Invalid value'], $result->errors);
    }
}
