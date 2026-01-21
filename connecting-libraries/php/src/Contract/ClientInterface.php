<?php

declare(strict_types=1);

namespace NeuroQuantum\Contract;

/**
 * Interface for API client services.
 */
interface ClientInterface
{
    /**
     * Get the connection instance.
     */
    public function getConnection(): ConnectionInterface;
}
