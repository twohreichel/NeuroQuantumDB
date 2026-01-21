<?php

declare(strict_types=1);

namespace NeuroQuantum\Client;

use NeuroQuantum\Contract\ClientInterface;
use NeuroQuantum\Contract\ConnectionInterface;

/**
 * Base class for API clients.
 */
abstract class AbstractClient implements ClientInterface
{
    public function __construct(
        protected readonly ConnectionInterface $connection,
    ) {
    }

    public function getConnection(): ConnectionInterface
    {
        return $this->connection;
    }
}
