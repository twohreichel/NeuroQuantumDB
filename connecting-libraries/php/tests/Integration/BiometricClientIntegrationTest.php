<?php

declare(strict_types=1);

namespace NeuroQuantum\Tests\Integration;

use Mockery;
use Mockery\Adapter\Phpunit\MockeryPHPUnitIntegration;
use Mockery\MockInterface;
use NeuroQuantum\Client\BiometricClient;
use NeuroQuantum\Contract\ConnectionInterface;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

/**
 * Integration tests for BiometricClient.
 */
final class BiometricClientIntegrationTest extends TestCase
{
    use MockeryPHPUnitIntegration;

    private ConnectionInterface&MockInterface $connection;
    private BiometricClient $client;

    protected function setUp(): void
    {
        $this->connection = Mockery::mock(ConnectionInterface::class);
        $this->client = new BiometricClient($this->connection);
    }

    protected function tearDown(): void
    {
        Mockery::close();
        parent::tearDown();
    }

    #[Test]
    public function it_enrolls_eeg_biometric(): void
    {
        $biometricData = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8];

        $this->connection
            ->expects('post')
            ->with('/biometric/eeg/enroll', [
                'user_id' => 'user_123',
                'biometric_data' => $biometricData,
                'type' => 'eeg',
            ])
            ->andReturn(['enrolled' => true]);

        $result = $this->client->enroll('user_123', $biometricData, 'eeg');

        $this->assertTrue($result);
    }

    #[Test]
    public function it_enrolls_default_biometric(): void
    {
        $biometricData = [0.5, 0.5, 0.5];

        $this->connection
            ->expects('post')
            ->with('/biometric/enroll', [
                'user_id' => 'user_456',
                'biometric_data' => $biometricData,
                'type' => 'fingerprint',
            ])
            ->andReturn(['enrolled' => true]);

        $result = $this->client->enroll('user_456', $biometricData, 'fingerprint');

        $this->assertTrue($result);
    }

    #[Test]
    public function it_verifies_biometric_data(): void
    {
        $biometricData = [0.1, 0.2, 0.3, 0.4];

        $this->connection
            ->expects('post')
            ->with('/biometric/verify', [
                'user_id' => 'user_123',
                'biometric_data' => $biometricData,
            ])
            ->andReturn([
                'verified' => true,
                'match_score' => 0.95,
                'threshold' => 0.8,
            ]);

        $result = $this->client->verify('user_123', $biometricData);

        $this->assertTrue($result['verified']);
        $this->assertSame(0.95, $result['match_score']);
    }

    #[Test]
    public function it_verifies_with_low_match_score(): void
    {
        $biometricData = [0.9, 0.8, 0.7];

        $this->connection
            ->expects('post')
            ->with('/biometric/verify', Mockery::any())
            ->andReturn([
                'verified' => false,
                'match_score' => 0.45,
                'threshold' => 0.8,
            ]);

        $result = $this->client->verify('user_123', $biometricData);

        $this->assertFalse($result['verified']);
        $this->assertSame(0.45, $result['match_score']);
    }

    #[Test]
    public function it_authenticates_eeg(): void
    {
        $eegData = array_fill(0, 64, 0.5); // 64-channel EEG data

        $this->connection
            ->expects('post')
            ->with('/biometric/eeg/authenticate', [
                'user_id' => 'user_123',
                'eeg_data' => $eegData,
            ])
            ->andReturn([
                'authenticated' => true,
                'confidence' => 0.98,
                'processing_time_ms' => 45,
            ]);

        $result = $this->client->authenticateEeg('user_123', $eegData);

        $this->assertTrue($result['authenticated']);
        $this->assertSame(0.98, $result['confidence']);
    }

    #[Test]
    public function it_fails_eeg_authentication(): void
    {
        $eegData = [0.1, 0.2, 0.3];

        $this->connection
            ->expects('post')
            ->with('/biometric/eeg/authenticate', Mockery::any())
            ->andReturn([
                'authenticated' => false,
                'confidence' => 0.35,
                'reason' => 'Pattern mismatch',
            ]);

        $result = $this->client->authenticateEeg('user_123', $eegData);

        $this->assertFalse($result['authenticated']);
        $this->assertSame('Pattern mismatch', $result['reason']);
    }

    #[Test]
    public function it_updates_eeg_signature(): void
    {
        $eegData = [0.2, 0.3, 0.4, 0.5];

        $this->connection
            ->expects('post')
            ->with('/biometric/eeg/update', [
                'user_id' => 'user_123',
                'eeg_data' => $eegData,
            ])
            ->andReturn(['updated' => true]);

        $result = $this->client->updateEegSignature('user_123', $eegData);

        $this->assertTrue($result);
    }

    #[Test]
    public function it_lists_eeg_users(): void
    {
        $this->connection
            ->expects('get')
            ->with('/biometric/eeg/users')
            ->andReturn([
                'users' => [
                    ['user_id' => 'user_1', 'enrolled_at' => '2026-01-01T10:00:00Z'],
                    ['user_id' => 'user_2', 'enrolled_at' => '2026-01-15T15:30:00Z'],
                ],
            ]);

        $users = $this->client->listEegUsers();

        $this->assertCount(2, $users);
        $this->assertSame('user_1', $users[0]['user_id']);
    }

    #[Test]
    public function it_handles_empty_user_list(): void
    {
        $this->connection
            ->expects('get')
            ->with('/biometric/eeg/users')
            ->andReturn([]);

        $users = $this->client->listEegUsers();

        $this->assertEmpty($users);
    }

    #[Test]
    public function it_defaults_to_eeg_type(): void
    {
        $biometricData = [0.1, 0.2, 0.3];

        $this->connection
            ->expects('post')
            ->with('/biometric/eeg/enroll', Mockery::on(fn($data) => $data['type'] === 'eeg'))
            ->andReturn(['enrolled' => true]);

        $result = $this->client->enroll('user_123', $biometricData);

        $this->assertTrue($result);
    }
}
