<?php

declare(strict_types=1);

namespace NeuroQuantum\Tests\Integration;

use Mockery;
use Mockery\Adapter\Phpunit\MockeryPHPUnitIntegration;
use Mockery\MockInterface;
use NeuroQuantum\Client\AuthClient;
use NeuroQuantum\Contract\ConnectionInterface;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

/**
 * Integration tests for AuthClient.
 */
final class AuthClientIntegrationTest extends TestCase
{
    use MockeryPHPUnitIntegration;

    private ConnectionInterface&MockInterface $connection;
    private AuthClient $client;

    protected function setUp(): void
    {
        $this->connection = Mockery::mock(ConnectionInterface::class);
        $this->client = new AuthClient($this->connection);
    }

    protected function tearDown(): void
    {
        Mockery::close();
        parent::tearDown();
    }

    #[Test]
    public function it_generates_api_key(): void
    {
        $this->connection
            ->expects('post')
            ->with('/auth/api-key/generate', [
                'name' => 'Production API Key',
                'permissions' => ['read'],
                'expiry_hours' => 720,
                'rate_limit_per_hour' => 1000,
            ])
            ->andReturn([
                'api_key' => 'nqdb_prod_abc123xyz',
                'name' => 'Production API Key',
                'expires_at' => '2026-02-20T00:00:00Z',
                'permissions' => ['read'],
            ]);

        $result = $this->client->generateApiKey('Production API Key');

        $this->assertStringStartsWith('nqdb_', $result['api_key']);
        $this->assertSame('Production API Key', $result['name']);
    }

    #[Test]
    public function it_generates_api_key_with_custom_permissions(): void
    {
        $permissions = ['read', 'write', 'admin'];

        $this->connection
            ->expects('post')
            ->with('/auth/api-key/generate', Mockery::on(fn($data) =>
                $data['permissions'] === $permissions))
            ->andReturn([
                'api_key' => 'nqdb_admin_xyz789',
                'permissions' => $permissions,
            ]);

        $result = $this->client->generateApiKey('Admin Key', $permissions);

        $this->assertSame($permissions, $result['permissions']);
    }

    #[Test]
    public function it_generates_api_key_with_custom_expiry(): void
    {
        $this->connection
            ->expects('post')
            ->with('/auth/api-key/generate', Mockery::on(fn($data) =>
                $data['expiry_hours'] === 24))
            ->andReturn([
                'api_key' => 'nqdb_temp_key123',
                'expires_at' => '2026-01-22T00:00:00Z',
            ]);

        $result = $this->client->generateApiKey('Temp Key', ['read'], 24);

        $this->assertArrayHasKey('api_key', $result);
    }

    #[Test]
    public function it_generates_api_key_with_custom_rate_limit(): void
    {
        $this->connection
            ->expects('post')
            ->with('/auth/api-key/generate', Mockery::on(fn($data) =>
                $data['rate_limit_per_hour'] === 10000))
            ->andReturn([
                'api_key' => 'nqdb_highrate_key',
                'rate_limit_per_hour' => 10000,
            ]);

        $result = $this->client->generateApiKey('High Rate Key', ['read'], 720, 10000);

        $this->assertSame(10000, $result['rate_limit_per_hour']);
    }

    #[Test]
    public function it_revokes_api_key(): void
    {
        $this->connection
            ->expects('post')
            ->with('/auth/api-key/revoke', [
                'api_key' => 'nqdb_old_key_to_revoke',
            ])
            ->andReturn(['revoked' => true]);

        $result = $this->client->revokeApiKey('nqdb_old_key_to_revoke');

        $this->assertTrue($result);
    }

    #[Test]
    public function it_generates_read_only_key(): void
    {
        $this->connection
            ->expects('post')
            ->with('/auth/api-key/generate', Mockery::on(fn($data) =>
                $data['permissions'] === ['read'] &&
                $data['name'] === 'Read-Only Analytics'))
            ->andReturn([
                'api_key' => 'nqdb_readonly_xyz',
                'permissions' => ['read'],
            ]);

        $result = $this->client->generateApiKey('Read-Only Analytics', ['read']);

        $this->assertSame(['read'], $result['permissions']);
    }

    #[Test]
    public function it_generates_write_key(): void
    {
        $this->connection
            ->expects('post')
            ->with('/auth/api-key/generate', Mockery::on(fn($data) =>
                $data['permissions'] === ['read', 'write']))
            ->andReturn([
                'api_key' => 'nqdb_readwrite_abc',
                'permissions' => ['read', 'write'],
            ]);

        $result = $this->client->generateApiKey('Service Account', ['read', 'write']);

        $this->assertSame(['read', 'write'], $result['permissions']);
    }
}
