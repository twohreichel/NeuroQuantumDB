<?php

declare(strict_types=1);

namespace NeuroQuantum\Tests\Integration;

use Mockery;
use Mockery\Adapter\Phpunit\MockeryPHPUnitIntegration;
use Mockery\MockInterface;
use NeuroQuantum\Client\TableSchema;
use NeuroQuantum\Contract\ConnectionInterface;
use NeuroQuantum\NeuroQuantumDB;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

/**
 * Integration tests for the main NeuroQuantumDB facade.
 */
final class NeuroQuantumDBIntegrationTest extends TestCase
{
    use MockeryPHPUnitIntegration;

    private ConnectionInterface&MockInterface $connection;
    private NeuroQuantumDB $db;

    protected function setUp(): void
    {
        $this->connection = Mockery::mock(ConnectionInterface::class);
        $this->db = new NeuroQuantumDB($this->connection);
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
                'rows' => [['id' => 1, 'name' => 'John']],
                'columns' => ['id', 'name'],
            ]);

        $result = $this->db->query('SELECT * FROM users');

        $this->assertCount(1, $result->rows);
    }

    #[Test]
    public function it_executes_select_and_returns_rows(): void
    {
        $this->connection
            ->expects('post')
            ->with('/query', ['query' => 'SELECT id, name FROM users'])
            ->andReturn([
                'rows' => [
                    ['id' => 1, 'name' => 'John'],
                    ['id' => 2, 'name' => 'Jane'],
                ],
                'columns' => ['id', 'name'],
            ]);

        $rows = $this->db->select('SELECT id, name FROM users');

        $this->assertCount(2, $rows);
        $this->assertSame('John', $rows[0]['name']);
    }

    #[Test]
    public function it_returns_first_row(): void
    {
        $this->connection
            ->expects('post')
            ->with('/query', ['query' => 'SELECT * FROM users WHERE id = 1'])
            ->andReturn([
                'rows' => [['id' => 1, 'name' => 'John', 'email' => 'john@example.com']],
                'columns' => ['id', 'name', 'email'],
            ]);

        $user = $this->db->selectOne('SELECT * FROM users WHERE id = 1');

        $this->assertNotNull($user);
        $this->assertSame(1, $user['id']);
        $this->assertSame('John', $user['name']);
    }

    #[Test]
    public function it_returns_null_when_no_rows(): void
    {
        $this->connection
            ->expects('post')
            ->with('/query', Mockery::any())
            ->andReturn(['rows' => [], 'columns' => []]);

        $user = $this->db->selectOne('SELECT * FROM users WHERE id = 999');

        $this->assertNull($user);
    }

    #[Test]
    public function it_creates_table_from_schema(): void
    {
        $schema = TableSchema::create('products')
            ->id()
            ->string('name')
            ->float('price')
            ->integer('quantity')
            ->timestamps();

        $this->connection
            ->expects('post')
            ->with('/tables', Mockery::on(fn($data) =>
                $data['schema']['name'] === 'products' &&
                count($data['schema']['columns']) >= 4))
            ->andReturn(['created' => true]);

        $result = $this->db->createTable($schema);

        $this->assertTrue($result);
    }

    #[Test]
    public function it_inserts_records(): void
    {
        $records = [['name' => 'John', 'email' => 'john@example.com']];

        $this->connection
            ->expects('post')
            ->with('/tables/insert', [
                'table_name' => 'users',
                'records' => $records,
            ])
            ->andReturn([
                'inserted_count' => 1,
                'failed_count' => 0,
                'inserted_ids' => ['1'],
            ]);

        $result = $this->db->insert('users', $records);

        $this->assertSame(1, $result->insertedCount);
        $this->assertSame('1', $result->getLastInsertId());
    }

    #[Test]
    public function it_inserts_multiple_records(): void
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

        $result = $this->db->insert('users', $records);

        $this->assertSame(2, $result->insertedCount);
    }

    #[Test]
    public function it_provides_table_query_builder(): void
    {
        $this->connection
            ->expects('post')
            ->with('/tables/query', Mockery::on(fn($data) =>
                $data['table_name'] === 'users' &&
                isset($data['filters']) &&
                isset($data['limit'])))
            ->andReturn([
                'rows' => [['id' => 1, 'name' => 'John', 'age' => 25]],
            ]);

        $rows = $this->db->table('users')
            ->where('age', '>', 21)
            ->limit(10)
            ->get();

        $this->assertCount(1, $rows);
    }

    #[Test]
    public function it_provides_table_query_builder_with_select(): void
    {
        $this->connection
            ->expects('post')
            ->with('/tables/query', Mockery::on(fn($data) =>
                $data['columns'] === ['id', 'name']))
            ->andReturn([
                'rows' => [['id' => 1, 'name' => 'John']],
            ]);

        $rows = $this->db->table('users')
            ->select(['id', 'name'])
            ->get();

        $this->assertCount(1, $rows);
    }

    #[Test]
    public function it_executes_transaction(): void
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

        $result = $this->db->transaction(function () {
            return 'success';
        });

        $this->assertSame('success', $result);
    }

    #[Test]
    public function it_accesses_quantum_client(): void
    {
        $queryVector = [0.1, 0.2, 0.3];

        $this->connection
            ->expects('post')
            ->with('/quantum/search', Mockery::any())
            ->andReturn(['results' => []]);

        $results = $this->db->quantum()->search('documents', $queryVector);

        $this->assertEmpty($results);
    }

    #[Test]
    public function it_accesses_neural_client(): void
    {
        $this->connection
            ->expects('post')
            ->with('/neural/train', Mockery::any())
            ->andReturn(['job_id' => 'job_123']);

        $jobId = $this->db->neural()->train('model', []);

        $this->assertSame('job_123', $jobId);
    }

    #[Test]
    public function it_accesses_dna_client(): void
    {
        $this->connection
            ->expects('post')
            ->with('/dna/compress', Mockery::any())
            ->andReturn(['compressed_data' => 'abc']);

        $result = $this->db->dna()->compress(['ATCG']);

        $this->assertArrayHasKey('compressed_data', $result);
    }

    #[Test]
    public function it_accesses_biometric_client(): void
    {
        $this->connection
            ->expects('post')
            ->with('/biometric/eeg/enroll', Mockery::any())
            ->andReturn(['enrolled' => true]);

        $result = $this->db->biometric()->enroll('user_1', [0.1, 0.2]);

        $this->assertTrue($result);
    }

    #[Test]
    public function it_accesses_stats_client(): void
    {
        $this->connection
            ->expects('get')
            ->with('/stats/performance')
            ->andReturn(['queries_per_second' => 1000]);

        $stats = $this->db->stats()->getPerformance();

        $this->assertSame(1000, $stats['queries_per_second']);
    }

    #[Test]
    public function it_accesses_auth_client(): void
    {
        $this->connection
            ->expects('post')
            ->with('/auth/api-key/generate', Mockery::any())
            ->andReturn(['api_key' => 'nqdb_new_key']);

        $result = $this->db->auth()->generateApiKey('New Key');

        $this->assertSame('nqdb_new_key', $result['api_key']);
    }

    #[Test]
    public function it_reuses_lazy_loaded_clients(): void
    {
        $quantum1 = $this->db->quantum();
        $quantum2 = $this->db->quantum();

        $this->assertSame($quantum1, $quantum2);
    }
}
