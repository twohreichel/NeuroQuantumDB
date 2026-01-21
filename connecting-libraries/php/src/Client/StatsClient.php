<?php

declare(strict_types=1);

namespace NeuroQuantum\Client;

/**
 * Client for stats and monitoring endpoints.
 */
final class StatsClient extends AbstractClient
{
    /**
     * Get performance statistics.
     *
     * @return array<string, mixed> Performance stats
     */
    public function getPerformance(): array
    {
        return $this->connection->get('/stats/performance');
    }

    /**
     * Get detailed metrics (admin only).
     *
     * @return array<string, mixed> Detailed metrics
     */
    public function getMetrics(): array
    {
        return $this->connection->get('/stats/metrics');
    }

    /**
     * Get index recommendations.
     *
     * @return array<int, array<string, mixed>> Index recommendations
     */
    public function getIndexRecommendations(): array
    {
        $response = $this->connection->get('/advisor/indexes');
        return $response['recommendations'] ?? [];
    }

    /**
     * Clear advisor statistics (admin only).
     *
     * @return bool True if cleared successfully
     */
    public function clearAdvisorStats(): bool
    {
        $this->connection->delete('/advisor/indexes');
        return true;
    }
}
