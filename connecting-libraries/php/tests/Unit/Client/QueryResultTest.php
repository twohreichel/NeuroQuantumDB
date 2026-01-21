<?php

declare(strict_types=1);

namespace NeuroQuantum\Tests\Unit\Client;

use NeuroQuantum\Client\QueryResult;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

final class QueryResultTest extends TestCase
{
    #[Test]
    public function it_creates_result_from_array(): void
    {
        $data = [
            'success' => true,
            'rows_affected' => 2,
            'rows' => [
                ['id' => 1, 'name' => 'Alice'],
                ['id' => 2, 'name' => 'Bob'],
            ],
            'columns' => ['id', 'name'],
            'execution_time_ms' => 5.5,
        ];

        $result = QueryResult::fromArray($data);

        $this->assertTrue($result->success);
        $this->assertSame(2, $result->rowsAffected);
        $this->assertCount(2, $result->rows);
        $this->assertSame(['id', 'name'], $result->columns);
        $this->assertSame(5.5, $result->executionTimeMs);
    }

    #[Test]
    public function it_gets_first_row(): void
    {
        $result = QueryResult::fromArray([
            'rows' => [
                ['id' => 1, 'name' => 'Alice'],
                ['id' => 2, 'name' => 'Bob'],
            ],
        ]);

        $first = $result->first();

        $this->assertSame(['id' => 1, 'name' => 'Alice'], $first);
    }

    #[Test]
    public function it_returns_null_when_no_rows(): void
    {
        $result = QueryResult::fromArray(['rows' => []]);

        $this->assertNull($result->first());
    }

    #[Test]
    public function it_gets_single_value(): void
    {
        $result = QueryResult::fromArray([
            'rows' => [['count' => 42]],
        ]);

        $this->assertSame(42, $result->value('count'));
    }

    #[Test]
    public function it_plucks_column(): void
    {
        $result = QueryResult::fromArray([
            'rows' => [
                ['id' => 1, 'name' => 'Alice'],
                ['id' => 2, 'name' => 'Bob'],
                ['id' => 3, 'name' => 'Charlie'],
            ],
        ]);

        $names = $result->pluck('name');

        $this->assertSame(['Alice', 'Bob', 'Charlie'], $names);
    }

    #[Test]
    public function it_checks_if_empty(): void
    {
        $emptyResult = QueryResult::fromArray(['rows' => []]);
        $nonEmptyResult = QueryResult::fromArray(['rows' => [['id' => 1]]]);

        $this->assertTrue($emptyResult->isEmpty());
        $this->assertFalse($nonEmptyResult->isEmpty());
    }

    #[Test]
    public function it_counts_rows(): void
    {
        $result = QueryResult::fromArray([
            'rows' => [
                ['id' => 1],
                ['id' => 2],
                ['id' => 3],
            ],
        ]);

        $this->assertSame(3, $result->count());
    }

    #[Test]
    public function it_handles_error_response(): void
    {
        $result = QueryResult::fromArray([
            'error' => 'Syntax error near...',
        ]);

        $this->assertFalse($result->success);
        $this->assertSame('Syntax error near...', $result->error);
    }
}
