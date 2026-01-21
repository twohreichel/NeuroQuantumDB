<?php

declare(strict_types=1);

namespace NeuroQuantum\Client;

/**
 * Client for neural network operations.
 */
final class NeuralClient extends AbstractClient
{
    /**
     * Train a neural network model.
     *
     * @param string $name Model name
     * @param array<int, array<string, mixed>> $trainingData Training data
     * @param array<string, mixed> $config Training configuration
     * @return string Training job ID
     */
    public function train(string $name, array $trainingData, array $config = []): string
    {
        $request = [
            'name' => $name,
            'training_data' => $trainingData,
            'config' => $config,
        ];

        $response = $this->connection->post('/neural/train', $request);
        return $response['job_id'] ?? '';
    }

    /**
     * Get training status.
     *
     * @param string $jobId Training job ID
     * @return array<string, mixed> Training status
     */
    public function getTrainingStatus(string $jobId): array
    {
        return $this->connection->get('/neural/status/' . $jobId);
    }

    /**
     * Wait for training to complete.
     *
     * @param string $jobId Training job ID
     * @param int $timeoutSeconds Maximum wait time
     * @param int $pollIntervalSeconds Poll interval
     * @return array<string, mixed> Final training status
     */
    public function waitForTraining(
        string $jobId,
        int $timeoutSeconds = 300,
        int $pollIntervalSeconds = 5,
    ): array {
        $startTime = time();

        while (time() - $startTime < $timeoutSeconds) {
            $status = $this->getTrainingStatus($jobId);

            if (isset($status['status']) && in_array($status['status'], ['completed', 'failed'], true)) {
                return $status;
            }

            sleep($pollIntervalSeconds);
        }

        return ['status' => 'timeout', 'job_id' => $jobId];
    }
}
