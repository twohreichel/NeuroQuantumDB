<?php

declare(strict_types=1);

namespace NeuroQuantum\Tests\Integration;

use Mockery;
use Mockery\Adapter\Phpunit\MockeryPHPUnitIntegration;
use Mockery\MockInterface;
use NeuroQuantum\Client\QueryClient;
use NeuroQuantum\Contract\ConnectionInterface;
use NeuroQuantum\Query\DeleteBuilder;
use NeuroQuantum\Query\FilterOperator;
use NeuroQuantum\Query\InsertBuilder;
use NeuroQuantum\Query\SelectBuilder;
use NeuroQuantum\Query\UpdateBuilder;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

/**
 * Integration tests for QueryClient.
 */
final class QueryClientIntegrationTest extends TestCase
{
    use MockeryPHPUnitIntegration;

    private ConnectionInterface&MockInterface $connection;
    private QueryClient $client;

    protected function setUp(): void
    {
        $this->connection = Mockery::mock(ConnectionInterface::class);
        $this->client = new QueryClient($this->connection);
    }

    protected function tearDown(): void
    {
        Mockery::close();
        parent::tearDown();
    }

    #[Test]
    public function it_executes_raw_query(): void
    {
        $this->connection
            ->expects('post')
            ->with('/query', ['query' => 'SELECT * FROM users'])
            ->andReturn([
                'rows' => [
                    ['id' => 1, 'name' => 'John'],
                    ['id' => 2, 'name' => 'Jane'],
                ],
                'columns' => ['id', 'name'],
                'rows_affected' => 0,
            ]);

        $result = $this->client->execute('SELECT * FROM users');

        $this->assertCount(2, $result->rows);
        $this->assertSame(['id', 'name'], $result->columns);
    }

    #[Test]
    public function it_executes_select_builder(): void
    {
        $builder = SelectBuilder::from('users')
            ->select(['id', 'name', 'email'])
            ->where('status', 'active')
            ->orderBy('name')
            ->limit(10);

        $this->connection
            ->expects('post')
            ->with('/query', Mockery::on(fn($arg) => str_contains($arg['query'], 'SELECT')))
            ->andReturn([
                'rows' => [['id' => 1, 'name' => 'John', 'email' => 'john@example.com']],
                'columns' => ['id', 'name', 'email'],
            ]);

        $rows = $this->client->select($builder);

        $this->assertCount(1, $rows);
        $this->assertSame('John', $rows[0]['name']);
    }

    #[Test]
    public function it_executes_insert_builder(): void
    {
        $builder = InsertBuilder::into('users')
            ->values([
                'name' => 'John Doe',
                'email' => 'john@example.com',
            ]);

        $this->connection
            ->expects('post')
            ->with('/query', Mockery::on(fn($arg) => str_contains($arg['query'], 'INSERT')))
            ->andReturn([
                'rows' => [],
                'columns' => [],
                'rows_affected' => 1,
            ]);

        $affected = $this->client->insert($builder);

        $this->assertSame(1, $affected);
    }

    #[Test]
    public function it_executes_update_builder(): void
    {
        $builder = UpdateBuilder::table('users')
            ->set('name', 'Jane Doe')
            ->set('updated_at', '2026-01-21')
            ->where('id', 1);

        $this->connection
            ->expects('post')
            ->with('/query', Mockery::on(fn($arg) => str_contains($arg['query'], 'UPDATE')))
            ->andReturn([
                'rows' => [],
                'columns' => [],
                'rows_affected' => 1,
            ]);

        $affected = $this->client->update($builder);

        $this->assertSame(1, $affected);
    }

    #[Test]
    public function it_executes_delete_builder(): void
    {
        $builder = DeleteBuilder::from('users')
            ->where('id', 1);

        $this->connection
            ->expects('post')
            ->with('/query', Mockery::on(fn($arg) => str_contains($arg['query'], 'DELETE')))
            ->andReturn([
                'rows' => [],
                'columns' => [],
                'rows_affected' => 1,
            ]);

        $affected = $this->client->delete($builder);

        $this->assertSame(1, $affected);
    }

    #[Test]
    public function it_begins_transaction(): void
    {
        $this->connection
            ->expects('post')
            ->with('/query', ['query' => 'BEGIN TRANSACTION'])
            ->andReturn(['rows' => [], 'columns' => []]);

        $this->client->beginTransaction();
    }

    #[Test]
    public function it_commits_transaction(): void
    {
        $this->connection
            ->expects('post')
            ->with('/query', ['query' => 'COMMIT'])
            ->andReturn(['rows' => [], 'columns' => []]);

        $this->client->commit();
    }

    #[Test]
    public function it_rollbacks_transaction(): void
    {
        $this->connection
            ->expects('post')
            ->with('/query', ['query' => 'ROLLBACK'])
            ->andReturn(['rows' => [], 'columns' => []]);

        $this->client->rollback();
    }

    #[Test]
    public function it_executes_callback_in_transaction(): void
    {
        $this->connection
            ->expects('post')
            ->with('/query', ['query' => 'BEGIN TRANSACTION'])
            ->once()
            ->andReturn(['rows' => [], 'columns' => []]);

        $this->connection
            ->expects('post')
            ->with('/query', ['query' => 'COMMIT'])
            ->once()
            ->andReturn(['rows' => [], 'columns' => []]);

        $result = $this->client->transaction(function () {
            return 'success';
        });

        $this->assertSame('success', $result);
    }

    #[Test]
    public function it_rolls_back_on_exception(): void
    {
        $this->connection
            ->expects('post')
            ->with('/query', ['query' => 'BEGIN TRANSACTION'])
            ->once()
            ->andReturn(['rows' => [], 'columns' => []]);

        $this->connection
            ->expects('post')
            ->with('/query', ['query' => 'ROLLBACK'])
            ->once()
            ->andReturn(['rows' => [], 'columns' => []]);

        $this->expectException(\RuntimeException::class);
        $this->expectExceptionMessage('Test error');

        $this->client->transaction(function () {
            throw new \RuntimeException('Test error');
        });
    }

    #[Test]
    public function it_executes_complex_select_with_joins(): void
    {
        $query = 'SELECT users.*, orders.total FROM users JOIN orders ON users.id = orders.user_id WHERE orders.total > 100';

        $this->connection
            ->expects('post')
            ->with('/query', ['query' => $query])
            ->andReturn([
                'rows' => [
                    ['id' => 1, 'name' => 'John', 'total' => 150.00],
                    ['id' => 2, 'name' => 'Jane', 'total' => 200.00],
                ],
                'columns' => ['id', 'name', 'total'],
            ]);

        $result = $this->client->execute($query);

        $this->assertCount(2, $result->rows);
        $this->assertSame(150.00, $result->rows[0]['total']);
    }
}
