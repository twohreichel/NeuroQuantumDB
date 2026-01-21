<?php

declare(strict_types=1);

namespace NeuroQuantum\Contract;

/**
 * Interface for query builders.
 */
interface QueryBuilderInterface
{
    /**
     * Build the query and return as array for API request.
     *
     * @return array<string, mixed>
     */
    public function toArray(): array;

    /**
     * Build raw SQL string representation.
     */
    public function toSql(): string;

    /**
     * Get bound parameters.
     *
     * @return array<string, mixed>
     */
    public function getBindings(): array;
}
