<?php

declare(strict_types=1);

namespace NeuroQuantum\Config;

use NeuroQuantum\Contract\ConfigInterface;
use NeuroQuantum\Exception\ConfigurationException;

/**
 * Configuration class for NeuroQuantumDB connection.
 *
 * Supports loading from environment variables, arrays, or direct parameters.
 */
final class Config implements ConfigInterface
{
    private const DEFAULT_HOST = 'localhost';
    private const DEFAULT_PORT = 8080;
    private const DEFAULT_TIMEOUT = 30;
    private const DEFAULT_MAX_RETRY_ATTEMPTS = 3;
    private const API_VERSION = 'v1';

    public function __construct(
        private readonly string $host = self::DEFAULT_HOST,
        private readonly int $port = self::DEFAULT_PORT,
        private readonly string $apiKey = '',
        private readonly bool $ssl = false,
        private readonly int $timeout = self::DEFAULT_TIMEOUT,
        private readonly bool $retryEnabled = true,
        private readonly int $maxRetryAttempts = self::DEFAULT_MAX_RETRY_ATTEMPTS,
        private readonly bool $debug = false,
    ) {
    }

    /**
     * Create configuration from environment variables.
     *
     * @throws ConfigurationException When required configuration is missing
     */
    public static function fromEnvironment(): self
    {
        $apiKey = self::getEnv('NEUROQUANTUM_API_KEY');
        if ($apiKey === null || $apiKey === '') {
            throw ConfigurationException::missingRequired('NEUROQUANTUM_API_KEY');
        }

        return new self(
            host: self::getEnv('NEUROQUANTUM_HOST') ?? self::DEFAULT_HOST,
            port: (int) (self::getEnv('NEUROQUANTUM_PORT') ?? self::DEFAULT_PORT),
            apiKey: $apiKey,
            ssl: filter_var(self::getEnv('NEUROQUANTUM_USE_SSL') ?? 'false', FILTER_VALIDATE_BOOLEAN),
            timeout: (int) (self::getEnv('NEUROQUANTUM_TIMEOUT') ?? self::DEFAULT_TIMEOUT),
            retryEnabled: filter_var(self::getEnv('NEUROQUANTUM_RETRY_ENABLED') ?? 'true', FILTER_VALIDATE_BOOLEAN),
            maxRetryAttempts: (int) (self::getEnv('NEUROQUANTUM_RETRY_MAX_ATTEMPTS') ?? self::DEFAULT_MAX_RETRY_ATTEMPTS),
            debug: filter_var(self::getEnv('NEUROQUANTUM_DEBUG') ?? 'false', FILTER_VALIDATE_BOOLEAN),
        );
    }

    /**
     * Create configuration from array.
     *
     * @param array<string, mixed> $config Configuration array
     * @throws ConfigurationException When required configuration is missing
     */
    public static function fromArray(array $config): self
    {
        $apiKey = $config['api_key'] ?? $config['apiKey'] ?? '';
        if ($apiKey === '') {
            throw ConfigurationException::missingRequired('api_key');
        }

        return new self(
            host: (string) ($config['host'] ?? self::DEFAULT_HOST),
            port: (int) ($config['port'] ?? self::DEFAULT_PORT),
            apiKey: (string) $apiKey,
            ssl: (bool) ($config['ssl'] ?? $config['use_ssl'] ?? false),
            timeout: (int) ($config['timeout'] ?? self::DEFAULT_TIMEOUT),
            retryEnabled: (bool) ($config['retry_enabled'] ?? $config['retryEnabled'] ?? true),
            maxRetryAttempts: (int) ($config['max_retry_attempts'] ?? $config['maxRetryAttempts'] ?? self::DEFAULT_MAX_RETRY_ATTEMPTS),
            debug: (bool) ($config['debug'] ?? false),
        );
    }

    public function getHost(): string
    {
        return $this->host;
    }

    public function getPort(): int
    {
        return $this->port;
    }

    public function getApiKey(): string
    {
        return $this->apiKey;
    }

    public function useSsl(): bool
    {
        return $this->ssl;
    }

    public function getTimeout(): int
    {
        return $this->timeout;
    }

    public function isRetryEnabled(): bool
    {
        return $this->retryEnabled;
    }

    public function getMaxRetryAttempts(): int
    {
        return $this->maxRetryAttempts;
    }

    public function isDebug(): bool
    {
        return $this->debug;
    }

    public function getBaseUrl(): string
    {
        $scheme = $this->ssl ? 'https' : 'http';
        return sprintf('%s://%s:%d/api/%s', $scheme, $this->host, $this->port, self::API_VERSION);
    }

    public function toArray(): array
    {
        return [
            'host' => $this->host,
            'port' => $this->port,
            'api_key' => '***REDACTED***',
            'ssl' => $this->ssl,
            'timeout' => $this->timeout,
            'retry_enabled' => $this->retryEnabled,
            'max_retry_attempts' => $this->maxRetryAttempts,
            'debug' => $this->debug,
            'base_url' => $this->getBaseUrl(),
        ];
    }

    /**
     * Get environment variable value.
     */
    private static function getEnv(string $key): ?string
    {
        $value = $_ENV[$key] ?? getenv($key);
        return $value !== false ? (string) $value : null;
    }
}
