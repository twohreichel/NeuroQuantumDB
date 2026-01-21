<?php

declare(strict_types=1);

namespace NeuroQuantum\Client;

/**
 * Client for biometric authentication operations.
 */
final class BiometricClient extends AbstractClient
{
    /**
     * Enroll biometric data.
     *
     * @param string $userId User identifier
     * @param array<int, float> $biometricData Biometric feature vector
     * @param string $type Biometric type (e.g., 'eeg', 'fingerprint')
     * @return bool True if enrollment successful
     */
    public function enroll(string $userId, array $biometricData, string $type = 'eeg'): bool
    {
        $endpoint = match ($type) {
            'eeg' => '/biometric/eeg/enroll',
            default => '/biometric/enroll',
        };

        $response = $this->connection->post($endpoint, [
            'user_id' => $userId,
            'biometric_data' => $biometricData,
            'type' => $type,
        ]);

        return isset($response['enrolled']) ? (bool) $response['enrolled'] : true;
    }

    /**
     * Verify biometric data against enrolled user.
     *
     * @param string $userId User identifier
     * @param array<int, float> $biometricData Biometric feature vector
     * @return array<string, mixed> Verification result with match score
     */
    public function verify(string $userId, array $biometricData): array
    {
        return $this->connection->post('/biometric/verify', [
            'user_id' => $userId,
            'biometric_data' => $biometricData,
        ]);
    }

    /**
     * Authenticate using EEG biometrics.
     *
     * @param string $userId User identifier
     * @param array<int, float> $eegData EEG signal data
     * @return array<string, mixed> Authentication result
     */
    public function authenticateEeg(string $userId, array $eegData): array
    {
        return $this->connection->post('/biometric/eeg/authenticate', [
            'user_id' => $userId,
            'eeg_data' => $eegData,
        ]);
    }

    /**
     * Update EEG signature for a user.
     *
     * @param string $userId User identifier
     * @param array<int, float> $eegData New EEG signal data
     * @return bool True if update successful
     */
    public function updateEegSignature(string $userId, array $eegData): bool
    {
        $response = $this->connection->post('/biometric/eeg/update', [
            'user_id' => $userId,
            'eeg_data' => $eegData,
        ]);

        return isset($response['updated']) ? (bool) $response['updated'] : true;
    }

    /**
     * List enrolled EEG users.
     *
     * @return array<int, array<string, mixed>> List of enrolled users
     */
    public function listEegUsers(): array
    {
        $response = $this->connection->get('/biometric/eeg/users');
        return $response['users'] ?? [];
    }
}
