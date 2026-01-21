<?php

declare(strict_types=1);

namespace NeuroQuantum\Tests\Integration;

use Mockery;
use Mockery\Adapter\Phpunit\MockeryPHPUnitIntegration;
use NeuroQuantum\Config\Config;
use NeuroQuantum\Contract\ConnectionInterface;
use NeuroQuantum\Exception\AuthenticationException;
use NeuroQuantum\Exception\ConnectionException;
use NeuroQuantum\NeuroQuantumDB;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

/**
 * Integration tests for connection handling.
 *
 * These tests verify the connection lifecycle and error handling.
 */
final class ConnectionIntegrationTest extends TestCase
{
    use MockeryPHPUnitIntegration;

    protected function tearDown(): void
    {
        Mockery::close();
        parent::tearDown();
    }

    #[Test]
    public function it_creates_lazy_connection(): void
    {
        $config = Config::fromArray([
            'host' => 'localhost',
            'port' => 8080,
            'api_key' => 'test_key',
        ]);

        $db = NeuroQuantumDB::createLazy($config);

        $this->assertInstanceOf(NeuroQuantumDB::class, $db);
    }

    #[Test]
    public function it_can_access_connection_from_db(): void
    {
        $connection = Mockery::mock(ConnectionInterface::class);
        $connection->allows('get')->andReturn(['success' => true]);
        $connection->allows('post')->andReturn(['rows' => [], 'columns' => []]);

        $db = new NeuroQuantumDB($connection);

        $this->assertInstanceOf(NeuroQuantumDB::class, $db);
    }

    #[Test]
    public function it_creates_config_from_environment(): void
    {
        // Environment variables are set in phpunit.xml
        $config = Config::fromEnvironment();

        $this->assertSame('localhost', $config->getHost());
        $this->assertSame(8080, $config->getPort());
        $this->assertSame('test_api_key', $config->getApiKey());
    }

    #[Test]
    public function it_creates_config_with_defaults(): void
    {
        $config = Config::fromArray([
            'api_key' => 'my_key',
        ]);

        $this->assertSame('localhost', $config->getHost());
        $this->assertSame(8080, $config->getPort());
        $this->assertSame(30, $config->getTimeout());
        $this->assertFalse($config->useSsl());
        $this->assertSame(3, $config->getMaxRetryAttempts());
    }

    #[Test]
    public function it_creates_config_with_ssl(): void
    {
        $config = Config::fromArray([
            'api_key' => 'my_key',
            'ssl' => true,
        ]);

        $this->assertTrue($config->useSsl());
    }
}
