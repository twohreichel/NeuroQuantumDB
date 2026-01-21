<?php

declare(strict_types=1);

namespace NeuroQuantum\Tests\Integration;

use Mockery;
use Mockery\Adapter\Phpunit\MockeryPHPUnitIntegration;
use Mockery\MockInterface;
use NeuroQuantum\Client\StatsClient;
use NeuroQuantum\Contract\ConnectionInterface;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

/**
 * Integration tests for StatsClient.
 */
final class StatsClientIntegrationTest extends TestCase
{
    use MockeryPHPUnitIntegration;

    private ConnectionInterface&MockInterface $connection;
    private StatsClient $client;

    protected function setUp(): void
    {
        $this->connection = Mockery::mock(ConnectionInterface::class);
        $this->client = new StatsClient($this->connection);
    }

    protected function tearDown(): void
    {
        Mockery::close();
        parent::tearDown();
    }

    #[Test]
    public function it_gets_performance_stats(): void
    {
        $this->connection
            ->expects('get')
            ->with('/stats/performance')
            ->andReturn([
                'queries_per_second' => 1500,
                'average_latency_ms' => 2.5,
                'active_connections' => 42,
                'memory_usage_mb' => 512,
                'uptime_seconds' => 86400,
            ]);

        $stats = $this->client->getPerformance();

        $this->assertSame(1500, $stats['queries_per_second']);
        $this->assertSame(2.5, $stats['average_latency_ms']);
        $this->assertSame(42, $stats['active_connections']);
    }

    #[Test]
    public function it_gets_detailed_metrics(): void
    {
        $this->connection
            ->expects('get')
            ->with('/stats/metrics')
            ->andReturn([
                'cpu_usage_percent' => 45.2,
                'disk_usage_percent' => 62.8,
                'cache_hit_rate' => 0.95,
                'slow_queries_count' => 5,
                'error_rate' => 0.001,
                'quantum_operations' => [
                    'total' => 10000,
                    'success_rate' => 0.998,
                ],
            ]);

        $metrics = $this->client->getMetrics();

        $this->assertSame(45.2, $metrics['cpu_usage_percent']);
        $this->assertSame(0.95, $metrics['cache_hit_rate']);
        $this->assertArrayHasKey('quantum_operations', $metrics);
    }

    #[Test]
    public function it_gets_index_recommendations(): void
    {
        $this->connection
            ->expects('get')
            ->with('/advisor/indexes')
            ->andReturn([
                'recommendations' => [
                    [
                        'table' => 'users',
                        'column' => 'email',
                        'index_type' => 'unique',
                        'reason' => 'Frequent lookups on email column',
                        'estimated_improvement' => 0.75,
                    ],
                    [
                        'table' => 'orders',
                        'column' => 'created_at',
                        'index_type' => 'btree',
                        'reason' => 'Frequent range queries on created_at',
                        'estimated_improvement' => 0.45,
                    ],
                ],
            ]);

        $recommendations = $this->client->getIndexRecommendations();

        $this->assertCount(2, $recommendations);
        $this->assertSame('users', $recommendations[0]['table']);
        $this->assertSame('email', $recommendations[0]['column']);
        $this->assertSame(0.75, $recommendations[0]['estimated_improvement']);
    }

    #[Test]
    public function it_handles_empty_recommendations(): void
    {
        $this->connection
            ->expects('get')
            ->with('/advisor/indexes')
            ->andReturn(['recommendations' => []]);

        $recommendations = $this->client->getIndexRecommendations();

        $this->assertEmpty($recommendations);
    }

    #[Test]
    public function it_handles_missing_recommendations_key(): void
    {
        $this->connection
            ->expects('get')
            ->with('/advisor/indexes')
            ->andReturn([]);

        $recommendations = $this->client->getIndexRecommendations();

        $this->assertEmpty($recommendations);
    }

    #[Test]
    public function it_clears_advisor_stats(): void
    {
        $this->connection
            ->expects('delete')
            ->with('/advisor/indexes')
            ->andReturn(['cleared' => true]);

        $result = $this->client->clearAdvisorStats();

        $this->assertTrue($result);
    }
}
