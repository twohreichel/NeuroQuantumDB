<?php

declare(strict_types=1);

namespace NeuroQuantum\Contract;

/**
 * Interface for HTTP connections to NeuroQuantumDB.
 */
interface ConnectionInterface
{
    /**
     * Send a GET request.
     *
     * @param string $endpoint API endpoint path
     * @param array<string, mixed> $query Query parameters
     * @return array<string, mixed> Decoded JSON response
     */
    public function get(string $endpoint, array $query = []): array;

    /**
     * Send a POST request.
     *
     * @param string $endpoint API endpoint path
     * @param array<string, mixed> $data Request body data
     * @return array<string, mixed> Decoded JSON response
     */
    public function post(string $endpoint, array $data = []): array;

    /**
     * Send a PUT request.
     *
     * @param string $endpoint API endpoint path
     * @param array<string, mixed> $data Request body data
     * @return array<string, mixed> Decoded JSON response
     */
    public function put(string $endpoint, array $data = []): array;

    /**
     * Send a DELETE request.
     *
     * @param string $endpoint API endpoint path
     * @param array<string, mixed> $data Request body data
     * @return array<string, mixed> Decoded JSON response
     */
    public function delete(string $endpoint, array $data = []): array;

    /**
     * Check if connection is established.
     */
    public function isConnected(): bool;

    /**
     * Get the base URL for the connection.
     */
    public function getBaseUrl(): string;
}
