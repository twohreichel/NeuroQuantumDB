<?php

declare(strict_types=1);

namespace NeuroQuantum\Query;

/**
 * Enum for filter operators matching NeuroQuantumDB API.
 */
enum FilterOperator: string
{
    case Equals = 'Equals';
    case NotEquals = 'NotEquals';
    case GreaterThan = 'GreaterThan';
    case LessThan = 'LessThan';
    case GreaterThanOrEquals = 'GreaterThanOrEquals';
    case LessThanOrEquals = 'LessThanOrEquals';
    case In = 'In';
    case NotIn = 'NotIn';
    case Like = 'Like';
    case NotLike = 'NotLike';
    case IsNull = 'IsNull';
    case IsNotNull = 'IsNotNull';
    case Contains = 'Contains';
    case StartsWith = 'StartsWith';
    case EndsWith = 'EndsWith';
    case NeuralSimilarity = 'NeuralSimilarity';
    case QuantumEntanglement = 'QuantumEntanglement';

    /**
     * Get SQL operator equivalent.
     */
    public function toSql(): string
    {
        return match ($this) {
            self::Equals => '=',
            self::NotEquals => '!=',
            self::GreaterThan => '>',
            self::LessThan => '<',
            self::GreaterThanOrEquals => '>=',
            self::LessThanOrEquals => '<=',
            self::In => 'IN',
            self::NotIn => 'NOT IN',
            self::Like => 'LIKE',
            self::NotLike => 'NOT LIKE',
            self::IsNull => 'IS NULL',
            self::IsNotNull => 'IS NOT NULL',
            self::Contains => 'LIKE',
            self::StartsWith => 'LIKE',
            self::EndsWith => 'LIKE',
            self::NeuralSimilarity => 'NEUROMATCH',
            self::QuantumEntanglement => 'QUANTUM_ENTANGLE',
        };
    }
}
