<?php

declare(strict_types=1);

namespace NeuroQuantum\Tests\Integration;

use Mockery;
use Mockery\Adapter\Phpunit\MockeryPHPUnitIntegration;
use Mockery\MockInterface;
use NeuroQuantum\Client\TableClient;
use NeuroQuantum\Contract\ConnectionInterface;
use NeuroQuantum\TableQueryBuilder;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

/**
 * Integration tests for TableQueryBuilder (fluent query interface).
 */
final class TableQueryBuilderIntegrationTest extends TestCase
{
    use MockeryPHPUnitIntegration;

    private ConnectionInterface&MockInterface $connection;
    private TableQueryBuilder $builder;

    protected function setUp(): void
    {
        $this->connection = Mockery::mock(ConnectionInterface::class);
        $client = new TableClient($this->connection);
        $this->builder = new TableQueryBuilder($client, 'users');
    }

    protected function tearDown(): void
    {
        Mockery::close();
        parent::tearDown();
    }

    #[Test]
    public function it_builds_simple_select(): void
    {
        $this->connection
            ->expects('post')
            ->with('/tables/query', Mockery::on(fn($data) => $data['table_name'] === 'users'))
            ->andReturn([
                'rows' => [
                    ['id' => 1, 'name' => 'John'],
                    ['id' => 2, 'name' => 'Jane'],
                ],
            ]);

        $rows = $this->builder->get();

        $this->assertCount(2, $rows);
    }

    #[Test]
    public function it_selects_specific_columns(): void
    {
        $this->connection
            ->expects('post')
            ->with('/tables/query', Mockery::on(fn($data) =>
                $data['columns'] === ['id', 'name', 'email']))
            ->andReturn(['rows' => []]);

        $this->builder->select(['id', 'name', 'email'])->get();
    }

    #[Test]
    public function it_filters_with_where(): void
    {
        $this->connection
            ->expects('post')
            ->with('/tables/query', Mockery::on(fn($data) =>
                isset($data['filters']) &&
                count($data['filters']) === 1))
            ->andReturn(['rows' => []]);

        $this->builder->where('status', 'active')->get();
    }

    #[Test]
    public function it_filters_with_operator(): void
    {
        $this->connection
            ->expects('post')
            ->with('/tables/query', Mockery::on(fn($data) =>
                isset($data['filters'])))
            ->andReturn(['rows' => []]);

        $this->builder->where('age', '>', 21)->get();
    }

    #[Test]
    public function it_filters_with_multiple_conditions(): void
    {
        $this->connection
            ->expects('post')
            ->with('/tables/query', Mockery::on(fn($data) =>
                isset($data['filters']) &&
                count($data['filters']) >= 2))
            ->andReturn(['rows' => []]);

        $this->builder
            ->where('status', 'active')
            ->where('age', '>=', 18)
            ->get();
    }

    #[Test]
    public function it_filters_with_where_in(): void
    {
        $this->connection
            ->expects('post')
            ->with('/tables/query', Mockery::on(fn($data) => isset($data['filters'])))
            ->andReturn(['rows' => []]);

        $this->builder->whereIn('id', [1, 2, 3])->get();
    }

    #[Test]
    public function it_filters_with_where_null(): void
    {
        $this->connection
            ->expects('post')
            ->with('/tables/query', Mockery::on(fn($data) => isset($data['filters'])))
            ->andReturn(['rows' => []]);

        $this->builder->whereNull('deleted_at')->get();
    }

    #[Test]
    public function it_filters_with_where_not_null(): void
    {
        $this->connection
            ->expects('post')
            ->with('/tables/query', Mockery::on(fn($data) => isset($data['filters'])))
            ->andReturn(['rows' => []]);

        $this->builder->whereNotNull('email_verified_at')->get();
    }

    #[Test]
    public function it_orders_results(): void
    {
        $this->connection
            ->expects('post')
            ->with('/tables/query', Mockery::on(fn($data) =>
                isset($data['sort']) &&
                count($data['sort']) > 0))
            ->andReturn(['rows' => []]);

        $this->builder->orderBy('name')->get();
    }

    #[Test]
    public function it_orders_descending(): void
    {
        $this->connection
            ->expects('post')
            ->with('/tables/query', Mockery::on(fn($data) =>
                isset($data['sort']) &&
                count($data['sort']) > 0))
            ->andReturn(['rows' => []]);

        $this->builder->orderByDesc('created_at')->get();
    }

    #[Test]
    public function it_limits_results(): void
    {
        $this->connection
            ->expects('post')
            ->with('/tables/query', Mockery::on(fn($data) => $data['limit'] === 10))
            ->andReturn(['rows' => []]);

        $this->builder->limit(10)->get();
    }

    #[Test]
    public function it_offsets_results(): void
    {
        $this->connection
            ->expects('post')
            ->with('/tables/query', Mockery::on(fn($data) => $data['offset'] === 20))
            ->andReturn(['rows' => []]);

        $this->builder->offset(20)->get();
    }

    #[Test]
    public function it_paginates_with_limit_and_offset(): void
    {
        $this->connection
            ->expects('post')
            ->with('/tables/query', Mockery::on(fn($data) =>
                $data['limit'] === 25 &&
                $data['offset'] === 50))
            ->andReturn(['rows' => []]);

        $this->builder->limit(25)->offset(50)->get();
    }

    #[Test]
    public function it_gets_first_result(): void
    {
        $this->connection
            ->expects('post')
            ->with('/tables/query', Mockery::on(fn($data) => $data['limit'] === 1))
            ->andReturn([
                'rows' => [['id' => 1, 'name' => 'John']],
            ]);

        $user = $this->builder->first();

        $this->assertNotNull($user);
        $this->assertSame(1, $user['id']);
    }

    #[Test]
    public function it_returns_null_when_first_not_found(): void
    {
        $this->connection
            ->expects('post')
            ->with('/tables/query', Mockery::any())
            ->andReturn(['rows' => []]);

        $user = $this->builder->first();

        $this->assertNull($user);
    }

    #[Test]
    public function it_counts_results(): void
    {
        $this->connection
            ->expects('post')
            ->with('/query', Mockery::on(fn($data) =>
                str_contains($data['query'], 'COUNT')))
            ->andReturn([
                'rows' => [['count' => 42]],
                'columns' => ['count'],
            ]);

        $count = $this->builder->count();

        $this->assertSame(42, $count);
    }

    #[Test]
    public function it_checks_existence(): void
    {
        $this->connection
            ->expects('post')
            ->with('/tables/query', Mockery::on(fn($data) => $data['limit'] === 1))
            ->andReturn([
                'rows' => [['id' => 1]],
            ]);

        $exists = $this->builder->where('email', 'john@example.com')->exists();

        $this->assertTrue($exists);
    }

    #[Test]
    public function it_checks_non_existence(): void
    {
        $this->connection
            ->expects('post')
            ->with('/tables/query', Mockery::on(fn($data) => $data['limit'] === 1))
            ->andReturn(['rows' => []]);

        $exists = $this->builder->where('email', 'nonexistent@example.com')->exists();

        $this->assertFalse($exists);
    }

    #[Test]
    public function it_updates_records(): void
    {
        $this->connection
            ->expects('put')
            ->with('/tables/update', Mockery::on(fn($data) =>
                isset($data['table_name']) &&
                $data['table_name'] === 'users' &&
                isset($data['updates']) &&
                isset($data['filters'])))
            ->andReturn(['updated_count' => 5]);

        $affected = $this->builder
            ->where('status', 'inactive')
            ->updateSet(['status' => 'archived']);

        $this->assertSame(5, $affected);
    }

    #[Test]
    public function it_deletes_records(): void
    {
        $this->connection
            ->expects('delete')
            ->with('/tables/delete', Mockery::on(fn($data) =>
                isset($data['table_name']) &&
                $data['table_name'] === 'users' &&
                isset($data['filters'])))
            ->andReturn(['deleted_count' => 3]);

        $affected = $this->builder
            ->where('status', 'deleted')
            ->deleteRows();

        $this->assertSame(3, $affected);
    }

    #[Test]
    public function it_chains_multiple_operations(): void
    {
        $this->connection
            ->expects('post')
            ->with('/tables/query', Mockery::on(fn($data) =>
                $data['table_name'] === 'users' &&
                $data['columns'] === ['id', 'name', 'email'] &&
                isset($data['filters']) &&
                isset($data['sort']) &&
                $data['limit'] === 10))
            ->andReturn([
                'rows' => [
                    ['id' => 1, 'name' => 'Alice', 'email' => 'alice@example.com'],
                    ['id' => 2, 'name' => 'Bob', 'email' => 'bob@example.com'],
                ],
            ]);

        $rows = $this->builder
            ->select(['id', 'name', 'email'])
            ->where('status', 'active')
            ->where('age', '>=', 18)
            ->orderBy('name')
            ->limit(10)
            ->get();

        $this->assertCount(2, $rows);
    }

    #[Test]
    public function it_plucks_single_column(): void
    {
        $this->connection
            ->expects('post')
            ->with('/tables/query', Mockery::any())
            ->andReturn([
                'rows' => [
                    ['id' => 1, 'name' => 'John'],
                    ['id' => 2, 'name' => 'Jane'],
                    ['id' => 3, 'name' => 'Bob'],
                ],
            ]);

        $names = $this->builder->pluck('name');

        $this->assertSame(['John', 'Jane', 'Bob'], $names);
    }

    #[Test]
    public function it_gets_single_value(): void
    {
        $this->connection
            ->expects('post')
            ->with('/tables/query', Mockery::on(fn($data) => $data['limit'] === 1))
            ->andReturn([
                'rows' => [['email' => 'john@example.com']],
            ]);

        $email = $this->builder->where('id', '=', 1)->value('email');

        $this->assertSame('john@example.com', $email);
    }

    #[Test]
    public function it_returns_null_for_missing_value(): void
    {
        $this->connection
            ->expects('post')
            ->with('/tables/query', Mockery::any())
            ->andReturn(['rows' => []]);

        $email = $this->builder->where('id', '=', 999)->value('email');

        $this->assertNull($email);
    }
}
