<?php

declare(strict_types=1);

namespace NeuroQuantum\Client;

/**
 * Client for API key management (admin only).
 */
final class AuthClient extends AbstractClient
{
    /**
     * Generate a new API key.
     *
     * @param string $name Key name/description
     * @param array<int, string> $permissions List of permissions
     * @param int $expiryHours Hours until key expires
     * @param int $rateLimitPerHour Rate limit per hour
     * @return array<string, mixed> API key response
     */
    public function generateApiKey(
        string $name,
        array $permissions = ['read'],
        int $expiryHours = 720,
        int $rateLimitPerHour = 1000,
    ): array {
        return $this->connection->post('/auth/api-key/generate', [
            'name' => $name,
            'permissions' => $permissions,
            'expiry_hours' => $expiryHours,
            'rate_limit_per_hour' => $rateLimitPerHour,
        ]);
    }

    /**
     * Revoke an API key.
     *
     * @param string $apiKey API key to revoke
     * @return bool True if revoked successfully
     */
    public function revokeApiKey(string $apiKey): bool
    {
        $this->connection->post('/auth/api-key/revoke', [
            'api_key' => $apiKey,
        ]);
        return true;
    }
}
