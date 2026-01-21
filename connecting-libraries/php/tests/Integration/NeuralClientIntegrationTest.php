<?php

declare(strict_types=1);

namespace NeuroQuantum\Tests\Integration;

use Mockery;
use Mockery\Adapter\Phpunit\MockeryPHPUnitIntegration;
use Mockery\MockInterface;
use NeuroQuantum\Client\NeuralClient;
use NeuroQuantum\Contract\ConnectionInterface;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

/**
 * Integration tests for NeuralClient.
 */
final class NeuralClientIntegrationTest extends TestCase
{
    use MockeryPHPUnitIntegration;

    private ConnectionInterface&MockInterface $connection;
    private NeuralClient $client;

    protected function setUp(): void
    {
        $this->connection = Mockery::mock(ConnectionInterface::class);
        $this->client = new NeuralClient($this->connection);
    }

    protected function tearDown(): void
    {
        Mockery::close();
        parent::tearDown();
    }

    #[Test]
    public function it_trains_neural_model(): void
    {
        $trainingData = [
            ['input' => [0.1, 0.2], 'output' => [1]],
            ['input' => [0.3, 0.4], 'output' => [0]],
        ];

        $this->connection
            ->expects('post')
            ->with('/neural/train', [
                'name' => 'classifier_v1',
                'training_data' => $trainingData,
                'config' => [],
            ])
            ->andReturn(['job_id' => 'job_12345']);

        $jobId = $this->client->train('classifier_v1', $trainingData);

        $this->assertSame('job_12345', $jobId);
    }

    #[Test]
    public function it_trains_with_custom_config(): void
    {
        $trainingData = [['input' => [0.1], 'output' => [1]]];
        $config = [
            'epochs' => 100,
            'learning_rate' => 0.001,
            'batch_size' => 32,
        ];

        $this->connection
            ->expects('post')
            ->with('/neural/train', Mockery::on(fn($data) =>
                $data['config']['epochs'] === 100 &&
                $data['config']['learning_rate'] === 0.001))
            ->andReturn(['job_id' => 'job_67890']);

        $jobId = $this->client->train('model', $trainingData, $config);

        $this->assertSame('job_67890', $jobId);
    }

    #[Test]
    public function it_gets_training_status(): void
    {
        $this->connection
            ->expects('get')
            ->with('/neural/status/job_12345')
            ->andReturn([
                'job_id' => 'job_12345',
                'status' => 'running',
                'progress' => 45,
                'current_epoch' => 45,
                'total_epochs' => 100,
            ]);

        $status = $this->client->getTrainingStatus('job_12345');

        $this->assertSame('running', $status['status']);
        $this->assertSame(45, $status['progress']);
    }

    #[Test]
    public function it_gets_completed_training_status(): void
    {
        $this->connection
            ->expects('get')
            ->with('/neural/status/job_12345')
            ->andReturn([
                'job_id' => 'job_12345',
                'status' => 'completed',
                'progress' => 100,
                'accuracy' => 0.95,
                'loss' => 0.05,
            ]);

        $status = $this->client->getTrainingStatus('job_12345');

        $this->assertSame('completed', $status['status']);
        $this->assertSame(0.95, $status['accuracy']);
    }

    #[Test]
    public function it_gets_failed_training_status(): void
    {
        $this->connection
            ->expects('get')
            ->with('/neural/status/job_failed')
            ->andReturn([
                'job_id' => 'job_failed',
                'status' => 'failed',
                'error' => 'Out of memory',
            ]);

        $status = $this->client->getTrainingStatus('job_failed');

        $this->assertSame('failed', $status['status']);
        $this->assertSame('Out of memory', $status['error']);
    }

    #[Test]
    public function it_returns_empty_job_id_on_missing_response(): void
    {
        $this->connection
            ->expects('post')
            ->with('/neural/train', Mockery::any())
            ->andReturn([]);

        $jobId = $this->client->train('model', []);

        $this->assertSame('', $jobId);
    }
}
