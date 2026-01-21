<?php

declare(strict_types=1);

namespace NeuroQuantum\Client;

/**
 * Client for quantum operations.
 */
final class QuantumClient extends AbstractClient
{
    /**
     * Perform quantum similarity search.
     *
     * @param string $table Table name
     * @param array<int, float> $queryVector Query vector for similarity search
     * @param float $threshold Similarity threshold (0.0 - 1.0)
     * @param int $maxResults Maximum number of results
     * @param array<string, mixed> $options Additional options
     * @return array<int, array<string, mixed>> Search results
     */
    public function search(
        string $table,
        array $queryVector,
        float $threshold = 0.7,
        int $maxResults = 10,
        array $options = [],
    ): array {
        $request = [
            'table_name' => $table,
            'query_vector' => $queryVector,
            'similarity_threshold' => $threshold,
            'max_results' => $maxResults,
        ];

        // Optional Grover search config
        if (isset($options['use_grover']) && $options['use_grover']) {
            $request['use_grover'] = true;
            if (isset($options['grover_config'])) {
                $request['grover_config'] = $options['grover_config'];
            }
        }

        // Optional TFIM config
        if (isset($options['use_tfim'])) {
            $request['use_tfim'] = (bool) $options['use_tfim'];
        }

        // Optional QUBO config
        if (isset($options['use_qubo'])) {
            $request['use_qubo'] = (bool) $options['use_qubo'];
        }

        // Entanglement boost
        if (isset($options['entanglement_boost'])) {
            $request['entanglement_boost'] = (float) $options['entanglement_boost'];
        }

        $response = $this->connection->post('/quantum/search', $request);
        return $response['results'] ?? [];
    }

    /**
     * Perform Grover's algorithm search.
     *
     * @param string $table Table name
     * @param array<int, float> $queryVector Query vector
     * @param int $numShots Number of measurement shots
     * @param bool $errorMitigation Enable error mitigation
     * @return array<int, array<string, mixed>> Search results
     */
    public function groverSearch(
        string $table,
        array $queryVector,
        int $numShots = 1024,
        bool $errorMitigation = true,
    ): array {
        return $this->search($table, $queryVector, 0.7, 10, [
            'use_grover' => true,
            'grover_config' => [
                'backend' => 'simulator',
                'num_shots' => $numShots,
                'error_mitigation' => $errorMitigation,
            ],
        ]);
    }
}
