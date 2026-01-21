<?php

declare(strict_types=1);

namespace NeuroQuantum\Tests\Integration;

use Mockery;
use Mockery\Adapter\Phpunit\MockeryPHPUnitIntegration;
use Mockery\MockInterface;
use NeuroQuantum\Client\QuantumClient;
use NeuroQuantum\Contract\ConnectionInterface;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

/**
 * Integration tests for QuantumClient.
 */
final class QuantumClientIntegrationTest extends TestCase
{
    use MockeryPHPUnitIntegration;

    private ConnectionInterface&MockInterface $connection;
    private QuantumClient $client;

    protected function setUp(): void
    {
        $this->connection = Mockery::mock(ConnectionInterface::class);
        $this->client = new QuantumClient($this->connection);
    }

    protected function tearDown(): void
    {
        Mockery::close();
        parent::tearDown();
    }

    #[Test]
    public function it_performs_quantum_similarity_search(): void
    {
        $queryVector = [0.1, 0.2, 0.3, 0.4, 0.5];

        $this->connection
            ->expects('post')
            ->with('/quantum/search', [
                'table_name' => 'documents',
                'query_vector' => $queryVector,
                'similarity_threshold' => 0.7,
                'max_results' => 10,
            ])
            ->andReturn([
                'results' => [
                    ['id' => 1, 'similarity' => 0.95, 'content' => 'Document 1'],
                    ['id' => 2, 'similarity' => 0.85, 'content' => 'Document 2'],
                ],
            ]);

        $results = $this->client->search('documents', $queryVector);

        $this->assertCount(2, $results);
        $this->assertSame(0.95, $results[0]['similarity']);
    }

    #[Test]
    public function it_performs_search_with_custom_threshold(): void
    {
        $queryVector = [0.1, 0.2, 0.3];

        $this->connection
            ->expects('post')
            ->with('/quantum/search', Mockery::on(fn($data) =>
                $data['similarity_threshold'] === 0.9 &&
                $data['max_results'] === 5))
            ->andReturn(['results' => []]);

        $results = $this->client->search('documents', $queryVector, 0.9, 5);

        $this->assertEmpty($results);
    }

    #[Test]
    public function it_performs_grover_search(): void
    {
        $queryVector = [0.1, 0.2, 0.3, 0.4, 0.5];

        $this->connection
            ->expects('post')
            ->with('/quantum/search', Mockery::on(fn($data) =>
                $data['use_grover'] === true &&
                isset($data['grover_config']) &&
                $data['grover_config']['num_shots'] === 1024 &&
                $data['grover_config']['error_mitigation'] === true))
            ->andReturn([
                'results' => [
                    ['id' => 1, 'similarity' => 0.99],
                ],
            ]);

        $results = $this->client->groverSearch('documents', $queryVector);

        $this->assertCount(1, $results);
    }

    #[Test]
    public function it_performs_grover_search_with_custom_shots(): void
    {
        $queryVector = [0.5, 0.5, 0.5];

        $this->connection
            ->expects('post')
            ->with('/quantum/search', Mockery::on(fn($data) =>
                $data['grover_config']['num_shots'] === 2048 &&
                $data['grover_config']['error_mitigation'] === false))
            ->andReturn(['results' => []]);

        $results = $this->client->groverSearch('documents', $queryVector, 2048, false);

        $this->assertEmpty($results);
    }

    #[Test]
    public function it_uses_tfim_option(): void
    {
        $queryVector = [0.1, 0.2, 0.3];

        $this->connection
            ->expects('post')
            ->with('/quantum/search', Mockery::on(fn($data) => $data['use_tfim'] === true))
            ->andReturn(['results' => []]);

        $results = $this->client->search('documents', $queryVector, 0.7, 10, ['use_tfim' => true]);

        $this->assertEmpty($results);
    }

    #[Test]
    public function it_uses_qubo_option(): void
    {
        $queryVector = [0.1, 0.2, 0.3];

        $this->connection
            ->expects('post')
            ->with('/quantum/search', Mockery::on(fn($data) => $data['use_qubo'] === true))
            ->andReturn(['results' => []]);

        $results = $this->client->search('documents', $queryVector, 0.7, 10, ['use_qubo' => true]);

        $this->assertEmpty($results);
    }

    #[Test]
    public function it_uses_entanglement_boost(): void
    {
        $queryVector = [0.1, 0.2, 0.3];

        $this->connection
            ->expects('post')
            ->with('/quantum/search', Mockery::on(fn($data) =>
                isset($data['entanglement_boost']) &&
                abs($data['entanglement_boost'] - 1.5) < 0.001))
            ->andReturn(['results' => []]);

        $results = $this->client->search('documents', $queryVector, 0.7, 10, ['entanglement_boost' => 1.5]);

        $this->assertEmpty($results);
    }

    #[Test]
    public function it_handles_empty_results(): void
    {
        $queryVector = [0.1, 0.2, 0.3];

        $this->connection
            ->expects('post')
            ->with('/quantum/search', Mockery::any())
            ->andReturn([]);

        $results = $this->client->search('documents', $queryVector);

        $this->assertEmpty($results);
    }
}
