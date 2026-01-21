<?php

declare(strict_types=1);

namespace NeuroQuantum\Tests\Integration;

use Mockery;
use Mockery\Adapter\Phpunit\MockeryPHPUnitIntegration;
use Mockery\MockInterface;
use NeuroQuantum\Client\TableClient;
use NeuroQuantum\Client\TableSchema;
use NeuroQuantum\Contract\ConnectionInterface;
use NeuroQuantum\Query\FilterOperator;
use NeuroQuantum\Query\SelectBuilder;
use NeuroQuantum\Query\SortDirection;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

/**
 * Integration tests for TableClient.
 */
final class TableClientIntegrationTest extends TestCase
{
    use MockeryPHPUnitIntegration;

    private ConnectionInterface&MockInterface $connection;
    private TableClient $client;

    protected function setUp(): void
    {
        $this->connection = Mockery::mock(ConnectionInterface::class);
        $this->client = new TableClient($this->connection);
    }

    protected function tearDown(): void
    {
        Mockery::close();
        parent::tearDown();
    }

    #[Test]
    public function it_creates_table_from_schema(): void
    {
        $schema = TableSchema::create('users')
            ->id()
            ->string('name')
            ->string('email')
            ->integer('age', true) // nullable
            ->boolean('is_active')
            ->timestamps();

        $this->connection
            ->expects('post')
            ->with('/tables', Mockery::on(function ($data) {
                return isset($data['schema']['name'])
                    && $data['schema']['name'] === 'users'
                    && isset($data['schema']['columns'])
                    && count($data['schema']['columns']) >= 5;
            }))
            ->andReturn(['created' => true]);

        $result = $this->client->create($schema);

        $this->assertTrue($result);
    }

    #[Test]
    public function it_creates_table_if_not_exists(): void
    {
        $schema = TableSchema::create('logs')->id()->text('message');

        $this->connection
            ->expects('post')
            ->with('/tables', Mockery::on(fn($data) => $data['if_not_exists'] === true))
            ->andReturn(['created' => true]);

        $result = $this->client->create($schema, ifNotExists: true);

        $this->assertTrue($result);
    }

    #[Test]
    public function it_drops_table(): void
    {
        $this->connection
            ->expects('post')
            ->with('/query', ['query' => 'DROP TABLE "users"'])
            ->andReturn(['rows' => [], 'columns' => [], 'rows_affected' => 0]);

        $result = $this->client->drop('users');

        $this->assertTrue($result);
    }

    #[Test]
    public function it_drops_table_if_exists(): void
    {
        $this->connection
            ->expects('post')
            ->with('/query', ['query' => 'DROP TABLE IF EXISTS "sessions"'])
            ->andReturn(['rows' => [], 'columns' => [], 'rows_affected' => 0]);

        $result = $this->client->drop('sessions', ifExists: true);

        $this->assertTrue($result);
    }

    #[Test]
    public function it_truncates_table(): void
    {
        $this->connection
            ->expects('post')
            ->with('/query', ['query' => 'TRUNCATE TABLE "logs"'])
            ->andReturn(['rows' => [], 'columns' => [], 'rows_affected' => 0]);

        $result = $this->client->truncate('logs');

        $this->assertTrue($result);
    }

    #[Test]
    public function it_inserts_records(): void
    {
        $records = [
            ['name' => 'John', 'email' => 'john@example.com'],
            ['name' => 'Jane', 'email' => 'jane@example.com'],
        ];

        $this->connection
            ->expects('post')
            ->with('/tables/insert', [
                'table_name' => 'users',
                'records' => $records,
            ])
            ->andReturn([
                'inserted_count' => 2,
                'failed_count' => 0,
                'inserted_ids' => [1, 2],
            ]);

        $result = $this->client->insert('users', $records);

        $this->assertSame(2, $result->insertedCount);
        $this->assertSame(0, $result->failedCount);
        $this->assertSame([1, 2], $result->insertedIds);
    }

    #[Test]
    public function it_inserts_single_record(): void
    {
        $record = [['name' => 'John', 'email' => 'john@example.com']];

        $this->connection
            ->expects('post')
            ->with('/tables/insert', [
                'table_name' => 'users',
                'records' => $record,
            ])
            ->andReturn([
                'inserted_count' => 1,
                'failed_count' => 0,
                'inserted_ids' => ['1'],
            ]);

        $result = $this->client->insert('users', $record);

        $this->assertSame(1, $result->insertedCount);
        $this->assertSame('1', $result->getLastInsertId());
    }

    #[Test]
    public function it_queries_all_records(): void
    {
        $this->connection
            ->expects('post')
            ->with('/tables/query', ['table_name' => 'users'])
            ->andReturn([
                'rows' => [
                    ['id' => 1, 'name' => 'John'],
                    ['id' => 2, 'name' => 'Jane'],
                ],
            ]);

        $rows = $this->client->query('users');

        $this->assertCount(2, $rows);
    }

    #[Test]
    public function it_queries_with_filters(): void
    {
        $filters = [
            'status' => ['operator' => '=', 'value' => 'active'],
            'age' => ['operator' => '>', 'value' => 18],
        ];

        $this->connection
            ->expects('post')
            ->with('/tables/query', [
                'table_name' => 'users',
                'filters' => $filters,
            ])
            ->andReturn([
                'rows' => [['id' => 1, 'name' => 'John', 'status' => 'active', 'age' => 25]],
            ]);

        $rows = $this->client->query('users', $filters);

        $this->assertCount(1, $rows);
    }

    #[Test]
    public function it_queries_with_sorting(): void
    {
        $sort = [
            ['column' => 'created_at', 'direction' => 'desc'],
            ['column' => 'name', 'direction' => 'asc'],
        ];

        $this->connection
            ->expects('post')
            ->with('/tables/query', [
                'table_name' => 'users',
                'sort' => $sort,
            ])
            ->andReturn([
                'rows' => [['id' => 2, 'name' => 'Jane'], ['id' => 1, 'name' => 'John']],
            ]);

        $rows = $this->client->query('users', [], $sort);

        $this->assertSame('Jane', $rows[0]['name']);
    }

    #[Test]
    public function it_queries_with_pagination(): void
    {
        $this->connection
            ->expects('post')
            ->with('/tables/query', [
                'table_name' => 'users',
                'limit' => 10,
                'offset' => 20,
            ])
            ->andReturn([
                'rows' => [['id' => 21, 'name' => 'User 21']],
            ]);

        $rows = $this->client->query('users', [], null, 10, 20);

        $this->assertCount(1, $rows);
        $this->assertSame(21, $rows[0]['id']);
    }

    #[Test]
    public function it_queries_specific_columns(): void
    {
        $this->connection
            ->expects('post')
            ->with('/tables/query', [
                'table_name' => 'users',
                'columns' => ['id', 'name'],
            ])
            ->andReturn([
                'rows' => [['id' => 1, 'name' => 'John']],
            ]);

        $rows = $this->client->query('users', [], null, null, 0, ['id', 'name']);

        $this->assertCount(1, $rows);
        $this->assertArrayHasKey('id', $rows[0]);
        $this->assertArrayHasKey('name', $rows[0]);
    }

    #[Test]
    public function it_selects_with_builder(): void
    {
        $builder = SelectBuilder::from('users')
            ->select(['id', 'name', 'email'])
            ->where('status', 'active')
            ->orderBy('name')
            ->limit(10);

        $this->connection
            ->expects('post')
            ->with('/tables/query', Mockery::on(fn($data) =>
                $data['table_name'] === 'users' &&
                isset($data['columns']) &&
                isset($data['filters']) &&
                isset($data['limit'])))
            ->andReturn([
                'rows' => [['id' => 1, 'name' => 'John', 'email' => 'john@example.com']],
            ]);

        $rows = $this->client->select($builder);

        $this->assertCount(1, $rows);
    }

    #[Test]
    public function it_updates_records(): void
    {
        $updates = ['name' => 'Jane Doe', 'updated_at' => '2026-01-21'];
        $filters = ['id' => ['operator' => '=', 'value' => 1]];

        $this->connection
            ->expects('put')
            ->with('/tables/update', [
                'table_name' => 'users',
                'updates' => $updates,
                'filters' => $filters,
            ])
            ->andReturn(['updated_count' => 1]);

        $affected = $this->client->update('users', $updates, $filters);

        $this->assertSame(1, $affected);
    }

    #[Test]
    public function it_deletes_records(): void
    {
        $filters = ['id' => ['operator' => '=', 'value' => 1]];

        $this->connection
            ->expects('delete')
            ->with('/tables/delete', [
                'table_name' => 'users',
                'filters' => $filters,
            ])
            ->andReturn(['deleted_count' => 1]);

        $affected = $this->client->delete('users', $filters);

        $this->assertSame(1, $affected);
    }

    #[Test]
    public function it_finds_record_by_id(): void
    {
        $this->connection
            ->expects('post')
            ->with('/tables/query', Mockery::on(fn($data) =>
                $data['table_name'] === 'users' &&
                isset($data['filters']) &&
                $data['limit'] === 1))
            ->andReturn([
                'rows' => [['id' => 1, 'name' => 'John', 'email' => 'john@example.com']],
            ]);

        $user = $this->client->find('users', 1);

        $this->assertNotNull($user);
        $this->assertSame(1, $user['id']);
        $this->assertSame('John', $user['name']);
    }

    #[Test]
    public function it_returns_null_when_not_found(): void
    {
        $this->connection
            ->expects('post')
            ->with('/tables/query', Mockery::any())
            ->andReturn(['rows' => []]);

        $user = $this->client->find('users', 999);

        $this->assertNull($user);
    }

    #[Test]
    public function it_checks_if_record_exists(): void
    {
        $filters = ['id' => ['operator' => '=', 'value' => 1]];

        $this->connection
            ->expects('post')
            ->with('/tables/query', Mockery::any())
            ->andReturn(['rows' => [['1' => 1]]]);

        $exists = $this->client->exists('users', $filters);

        $this->assertTrue($exists);
    }

    #[Test]
    public function it_counts_records(): void
    {
        $this->connection
            ->expects('post')
            ->with('/query', Mockery::on(fn($data) => str_contains($data['query'], 'COUNT')))
            ->andReturn([
                'rows' => [['count' => 42]],
                'columns' => ['count'],
            ]);

        $count = $this->client->count('users');

        $this->assertSame(42, $count);
    }

    #[Test]
    public function it_counts_records_with_filter(): void
    {
        $this->connection
            ->expects('post')
            ->with('/query', Mockery::on(fn($data) =>
                str_contains($data['query'], 'COUNT') &&
                str_contains($data['query'], 'WHERE')))
            ->andReturn([
                'rows' => [['count' => 10]],
                'columns' => ['count'],
            ]);

        $count = $this->client->count('users', ['status' => ['operator' => 'Equals', 'value' => 'active']]);

        $this->assertSame(10, $count);
    }
}
