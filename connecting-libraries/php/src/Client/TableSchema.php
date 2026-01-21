<?php

declare(strict_types=1);

namespace NeuroQuantum\Client;

/**
 * Value object for table schema definition.
 */
final class TableSchema
{
    /** @var array<int, ColumnDefinition> */
    private array $columns = [];

    /** @var array<int, ConstraintDefinition> */
    private array $constraints = [];

    public function __construct(
        private readonly string $name,
    ) {
    }

    /**
     * Create a new table schema.
     */
    public static function create(string $name): self
    {
        return new self($name);
    }

    /**
     * Add an integer column.
     */
    public function integer(string $name, bool $nullable = false, bool $autoIncrement = false): self
    {
        $this->columns[] = new ColumnDefinition($name, 'Integer', $nullable, $autoIncrement);
        return $this;
    }

    /**
     * Add a big integer / serial column.
     */
    public function bigInteger(string $name, bool $nullable = false, bool $autoIncrement = false): self
    {
        $this->columns[] = new ColumnDefinition($name, 'Integer', $nullable, $autoIncrement);
        return $this;
    }

    /**
     * Add a float column.
     */
    public function float(string $name, bool $nullable = false): self
    {
        $this->columns[] = new ColumnDefinition($name, 'Float', $nullable);
        return $this;
    }

    /**
     * Add a text/string column.
     */
    public function text(string $name, bool $nullable = false): self
    {
        $this->columns[] = new ColumnDefinition($name, 'Text', $nullable);
        return $this;
    }

    /**
     * Alias for text column.
     */
    public function string(string $name, bool $nullable = false): self
    {
        return $this->text($name, $nullable);
    }

    /**
     * Add a boolean column.
     */
    public function boolean(string $name, bool $nullable = false): self
    {
        $this->columns[] = new ColumnDefinition($name, 'Boolean', $nullable);
        return $this;
    }

    /**
     * Add a timestamp/datetime column.
     */
    public function timestamp(string $name, bool $nullable = false): self
    {
        $this->columns[] = new ColumnDefinition($name, 'Timestamp', $nullable);
        return $this;
    }

    /**
     * Add a binary/blob column.
     */
    public function binary(string $name, bool $nullable = false): self
    {
        $this->columns[] = new ColumnDefinition($name, 'Binary', $nullable);
        return $this;
    }

    /**
     * Add a JSON column.
     */
    public function json(string $name, bool $nullable = false): self
    {
        $this->columns[] = new ColumnDefinition($name, 'Json', $nullable);
        return $this;
    }

    /**
     * Add a DNA sequence column.
     */
    public function dna(string $name, bool $nullable = false): self
    {
        $this->columns[] = new ColumnDefinition($name, 'Dna', $nullable);
        return $this;
    }

    /**
     * Add a neural embedding column.
     */
    public function neural(string $name, bool $nullable = false): self
    {
        $this->columns[] = new ColumnDefinition($name, 'Neural', $nullable);
        return $this;
    }

    /**
     * Add a quantum state column.
     */
    public function quantum(string $name, bool $nullable = false): self
    {
        $this->columns[] = new ColumnDefinition($name, 'Quantum', $nullable);
        return $this;
    }

    /**
     * Add auto-incrementing primary key.
     */
    public function id(string $name = 'id'): self
    {
        $this->integer($name, false, true);
        return $this->primaryKey($name);
    }

    /**
     * Add timestamps (created_at, updated_at).
     */
    public function timestamps(): self
    {
        $this->timestamp('created_at', true);
        $this->timestamp('updated_at', true);
        return $this;
    }

    /**
     * Add primary key constraint.
     *
     * @param string|array<int, string> $columns
     */
    public function primaryKey(string|array $columns, ?string $name = null): self
    {
        $cols = is_array($columns) ? $columns : [$columns];
        $name ??= 'pk_' . $this->name;
        $this->constraints[] = new ConstraintDefinition($name, 'PrimaryKey', $cols);
        return $this;
    }

    /**
     * Add unique constraint.
     *
     * @param string|array<int, string> $columns
     */
    public function unique(string|array $columns, ?string $name = null): self
    {
        $cols = is_array($columns) ? $columns : [$columns];
        $name ??= 'uq_' . $this->name . '_' . implode('_', $cols);
        $this->constraints[] = new ConstraintDefinition($name, 'Unique', $cols);
        return $this;
    }

    /**
     * Add index.
     *
     * @param string|array<int, string> $columns
     */
    public function index(string|array $columns, ?string $name = null): self
    {
        $cols = is_array($columns) ? $columns : [$columns];
        $name ??= 'idx_' . $this->name . '_' . implode('_', $cols);
        $this->constraints[] = new ConstraintDefinition($name, 'Index', $cols);
        return $this;
    }

    /**
     * Get the table name.
     */
    public function getName(): string
    {
        return $this->name;
    }

    /**
     * Convert to API array format.
     *
     * @return array<string, mixed>
     */
    public function toArray(): array
    {
        return [
            'name' => $this->name,
            'columns' => array_map(fn($c) => $c->toArray(), $this->columns),
            'constraints' => array_map(fn($c) => $c->toArray(), $this->constraints),
        ];
    }
}

/**
 * Value object for column definition.
 */
final readonly class ColumnDefinition
{
    public function __construct(
        public string $name,
        public string $dataType,
        public bool $nullable = false,
        public bool $autoIncrement = false,
        public mixed $default = null,
    ) {
    }

    /**
     * @return array<string, mixed>
     */
    public function toArray(): array
    {
        $arr = [
            'name' => $this->name,
            'data_type' => $this->dataType,
            'nullable' => $this->nullable,
        ];

        if ($this->autoIncrement) {
            $arr['auto_increment'] = true;
        }

        if ($this->default !== null) {
            $arr['default'] = $this->default;
        }

        return $arr;
    }
}

/**
 * Value object for constraint definition.
 */
final readonly class ConstraintDefinition
{
    /**
     * @param array<int, string> $columns
     */
    public function __construct(
        public string $name,
        public string $type,
        public array $columns,
    ) {
    }

    /**
     * @return array<string, mixed>
     */
    public function toArray(): array
    {
        return [
            'name' => $this->name,
            'constraint_type' => $this->type,
            'columns' => $this->columns,
        ];
    }
}
