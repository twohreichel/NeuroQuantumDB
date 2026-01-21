<?php

declare(strict_types=1);

namespace NeuroQuantum\Tests\Integration;

use Mockery;
use Mockery\Adapter\Phpunit\MockeryPHPUnitIntegration;
use Mockery\MockInterface;
use NeuroQuantum\Client\DnaClient;
use NeuroQuantum\Contract\ConnectionInterface;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

/**
 * Integration tests for DnaClient.
 */
final class DnaClientIntegrationTest extends TestCase
{
    use MockeryPHPUnitIntegration;

    private ConnectionInterface&MockInterface $connection;
    private DnaClient $client;

    protected function setUp(): void
    {
        $this->connection = Mockery::mock(ConnectionInterface::class);
        $this->client = new DnaClient($this->connection);
    }

    protected function tearDown(): void
    {
        Mockery::close();
        parent::tearDown();
    }

    #[Test]
    public function it_compresses_dna_sequences(): void
    {
        $sequences = ['ATCGATCGATCG', 'GCTAGCTAGCTA'];

        $this->connection
            ->expects('post')
            ->with('/dna/compress', [
                'sequences' => $sequences,
                'algorithm' => DnaClient::ALGORITHM_KMER,
                'compression_level' => 5,
            ])
            ->andReturn([
                'compressed_data' => 'base64encodeddata==',
                'original_size' => 24,
                'compressed_size' => 12,
                'compression_ratio' => 0.5,
            ]);

        $result = $this->client->compress($sequences);

        $this->assertArrayHasKey('compressed_data', $result);
        $this->assertSame(0.5, $result['compression_ratio']);
    }

    #[Test]
    public function it_compresses_with_neural_algorithm(): void
    {
        $sequences = ['ATCGATCGATCG'];

        $this->connection
            ->expects('post')
            ->with('/dna/compress', Mockery::on(fn($data) =>
                $data['algorithm'] === DnaClient::ALGORITHM_NEURAL))
            ->andReturn([
                'compressed_data' => 'neuralcompressed==',
                'compression_ratio' => 0.3,
            ]);

        $result = $this->client->compress($sequences, DnaClient::ALGORITHM_NEURAL);

        $this->assertSame(0.3, $result['compression_ratio']);
    }

    #[Test]
    public function it_compresses_with_quantum_inspired_algorithm(): void
    {
        $sequences = ['ATCGATCGATCG'];

        $this->connection
            ->expects('post')
            ->with('/dna/compress', Mockery::on(fn($data) =>
                $data['algorithm'] === DnaClient::ALGORITHM_QUANTUM))
            ->andReturn(['compressed_data' => 'quantumcompressed==']);

        $result = $this->client->compress($sequences, DnaClient::ALGORITHM_QUANTUM);

        $this->assertArrayHasKey('compressed_data', $result);
    }

    #[Test]
    public function it_compresses_with_hybrid_algorithm(): void
    {
        $sequences = ['ATCGATCGATCG'];

        $this->connection
            ->expects('post')
            ->with('/dna/compress', Mockery::on(fn($data) =>
                $data['algorithm'] === DnaClient::ALGORITHM_HYBRID &&
                $data['compression_level'] === 9))
            ->andReturn(['compressed_data' => 'hybridcompressed==']);

        $result = $this->client->compress($sequences, DnaClient::ALGORITHM_HYBRID, 9);

        $this->assertArrayHasKey('compressed_data', $result);
    }

    #[Test]
    public function it_decompresses_dna_data(): void
    {
        $compressedData = 'base64encodeddata==';

        $this->connection
            ->expects('post')
            ->with('/dna/decompress', [
                'compressed_data' => $compressedData,
                'algorithm' => DnaClient::ALGORITHM_KMER,
            ])
            ->andReturn([
                'sequences' => ['ATCGATCGATCG', 'GCTAGCTAGCTA'],
            ]);

        $sequences = $this->client->decompress($compressedData);

        $this->assertCount(2, $sequences);
        $this->assertSame('ATCGATCGATCG', $sequences[0]);
    }

    #[Test]
    public function it_decompresses_with_matching_algorithm(): void
    {
        $compressedData = 'neuralcompressed==';

        $this->connection
            ->expects('post')
            ->with('/dna/decompress', [
                'compressed_data' => $compressedData,
                'algorithm' => DnaClient::ALGORITHM_NEURAL,
            ])
            ->andReturn([
                'sequences' => ['ATCGATCG'],
            ]);

        $sequences = $this->client->decompress($compressedData, DnaClient::ALGORITHM_NEURAL);

        $this->assertCount(1, $sequences);
    }

    #[Test]
    public function it_handles_empty_decompress_result(): void
    {
        $this->connection
            ->expects('post')
            ->with('/dna/decompress', Mockery::any())
            ->andReturn([]);

        $sequences = $this->client->decompress('invalid');

        $this->assertEmpty($sequences);
    }

    #[Test]
    public function it_compresses_large_sequence_set(): void
    {
        $sequences = array_fill(0, 1000, 'ATCGATCGATCGATCGATCG');

        $this->connection
            ->expects('post')
            ->with('/dna/compress', Mockery::on(fn($data) => count($data['sequences']) === 1000))
            ->andReturn([
                'compressed_data' => 'largecompressed==',
                'original_size' => 20000,
                'compressed_size' => 500,
                'compression_ratio' => 0.025,
            ]);

        $result = $this->client->compress($sequences);

        $this->assertSame(0.025, $result['compression_ratio']);
    }
}
