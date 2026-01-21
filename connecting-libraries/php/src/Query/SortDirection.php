<?php

declare(strict_types=1);

namespace NeuroQuantum\Query;

/**
 * Enum for sort directions.
 */
enum SortDirection: string
{
    case Asc = 'Asc';
    case Desc = 'Desc';

    public function toSql(): string
    {
        return match ($this) {
            self::Asc => 'ASC',
            self::Desc => 'DESC',
        };
    }
}
