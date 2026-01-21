<?php

declare(strict_types=1);

namespace NeuroQuantum\Contract;

/**
 * Interface for configuration providers.
 */
interface ConfigInterface
{
    /**
     * Get the database host.
     */
    public function getHost(): string;

    /**
     * Get the database port.
     */
    public function getPort(): int;

    /**
     * Get the API key for authentication.
     */
    public function getApiKey(): string;

    /**
     * Check if SSL should be used.
     */
    public function useSsl(): bool;

    /**
     * Get connection timeout in seconds.
     */
    public function getTimeout(): int;

    /**
     * Check if retry is enabled.
     */
    public function isRetryEnabled(): bool;

    /**
     * Get maximum retry attempts.
     */
    public function getMaxRetryAttempts(): int;

    /**
     * Check if debug mode is enabled.
     */
    public function isDebug(): bool;

    /**
     * Get the full base URL.
     */
    public function getBaseUrl(): string;

    /**
     * Get configuration as array.
     *
     * @return array<string, mixed>
     */
    public function toArray(): array;
}
