<?php

declare(strict_types=1);

namespace NeuroQuantum\Exception;

/**
 * Exception for configuration errors.
 */
class ConfigurationException extends NeuroQuantumException
{
    /**
     * Create exception for missing required configuration.
     */
    public static function missingRequired(string $key): self
    {
        return new self(
            sprintf('Missing required configuration: %s', $key),
            context: ['missing_key' => $key]
        );
    }

    /**
     * Create exception for invalid configuration value.
     */
    public static function invalidValue(string $key, mixed $value, string $expected): self
    {
        return new self(
            sprintf('Invalid configuration value for "%s": expected %s', $key, $expected),
            context: ['key' => $key, 'value' => $value, 'expected' => $expected]
        );
    }
}
