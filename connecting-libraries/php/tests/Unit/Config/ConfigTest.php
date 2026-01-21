<?php

declare(strict_types=1);

namespace NeuroQuantum\Tests\Unit\Config;

use NeuroQuantum\Config\Config;
use NeuroQuantum\Exception\ConfigurationException;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

final class ConfigTest extends TestCase
{
    #[Test]
    public function it_creates_config_with_default_values(): void
    {
        $config = new Config(apiKey: 'nqdb_test_key');

        $this->assertSame('localhost', $config->getHost());
        $this->assertSame(8080, $config->getPort());
        $this->assertSame('nqdb_test_key', $config->getApiKey());
        $this->assertFalse($config->useSsl());
        $this->assertSame(30, $config->getTimeout());
        $this->assertTrue($config->isRetryEnabled());
        $this->assertSame(3, $config->getMaxRetryAttempts());
        $this->assertFalse($config->isDebug());
    }

    #[Test]
    public function it_creates_config_with_custom_values(): void
    {
        $config = new Config(
            host: 'db.example.com',
            port: 9090,
            apiKey: 'nqdb_custom_key',
            ssl: true,
            timeout: 60,
            retryEnabled: false,
            maxRetryAttempts: 5,
            debug: true,
        );

        $this->assertSame('db.example.com', $config->getHost());
        $this->assertSame(9090, $config->getPort());
        $this->assertSame('nqdb_custom_key', $config->getApiKey());
        $this->assertTrue($config->useSsl());
        $this->assertSame(60, $config->getTimeout());
        $this->assertFalse($config->isRetryEnabled());
        $this->assertSame(5, $config->getMaxRetryAttempts());
        $this->assertTrue($config->isDebug());
    }

    #[Test]
    public function it_generates_correct_base_url_without_ssl(): void
    {
        $config = new Config(
            host: 'localhost',
            port: 8080,
            apiKey: 'test',
        );

        $this->assertSame('http://localhost:8080/api/v1', $config->getBaseUrl());
    }

    #[Test]
    public function it_generates_correct_base_url_with_ssl(): void
    {
        $config = new Config(
            host: 'secure.example.com',
            port: 443,
            apiKey: 'test',
            ssl: true,
        );

        $this->assertSame('https://secure.example.com:443/api/v1', $config->getBaseUrl());
    }

    #[Test]
    public function it_creates_config_from_array(): void
    {
        $config = Config::fromArray([
            'host' => 'array.example.com',
            'port' => 7070,
            'api_key' => 'nqdb_array_key',
            'ssl' => true,
            'timeout' => 45,
            'retry_enabled' => false,
            'max_retry_attempts' => 2,
            'debug' => true,
        ]);

        $this->assertSame('array.example.com', $config->getHost());
        $this->assertSame(7070, $config->getPort());
        $this->assertSame('nqdb_array_key', $config->getApiKey());
        $this->assertTrue($config->useSsl());
        $this->assertSame(45, $config->getTimeout());
        $this->assertFalse($config->isRetryEnabled());
        $this->assertSame(2, $config->getMaxRetryAttempts());
        $this->assertTrue($config->isDebug());
    }

    #[Test]
    public function it_throws_exception_when_api_key_missing_from_array(): void
    {
        $this->expectException(ConfigurationException::class);
        $this->expectExceptionMessage('Missing required configuration: api_key');

        Config::fromArray([
            'host' => 'localhost',
            'port' => 8080,
        ]);
    }

    #[Test]
    public function it_converts_to_array_with_redacted_api_key(): void
    {
        $config = new Config(
            host: 'localhost',
            port: 8080,
            apiKey: 'nqdb_secret_key',
        );

        $array = $config->toArray();

        $this->assertSame('localhost', $array['host']);
        $this->assertSame(8080, $array['port']);
        $this->assertSame('***REDACTED***', $array['api_key']);
        $this->assertArrayHasKey('base_url', $array);
    }

    #[Test]
    public function it_supports_camel_case_keys_in_array(): void
    {
        $config = Config::fromArray([
            'host' => 'localhost',
            'apiKey' => 'nqdb_camel_key',
            'retryEnabled' => false,
            'maxRetryAttempts' => 1,
        ]);

        $this->assertSame('nqdb_camel_key', $config->getApiKey());
        $this->assertFalse($config->isRetryEnabled());
        $this->assertSame(1, $config->getMaxRetryAttempts());
    }
}
