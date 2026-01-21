<?php

declare(strict_types=1);

namespace NeuroQuantum\Tests\Unit\Client;

use NeuroQuantum\Client\TableSchema;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

final class TableSchemaTest extends TestCase
{
    #[Test]
    public function it_creates_simple_schema(): void
    {
        $schema = TableSchema::create('users')
            ->integer('id', false, true)
            ->text('name')
            ->text('email')
            ->primaryKey('id');

        $array = $schema->toArray();

        $this->assertSame('users', $array['name']);
        $this->assertCount(3, $array['columns']);
        $this->assertCount(1, $array['constraints']);
    }

    #[Test]
    public function it_adds_integer_column(): void
    {
        $schema = TableSchema::create('test')
            ->integer('age', true);

        $array = $schema->toArray();
        $column = $array['columns'][0];

        $this->assertSame('age', $column['name']);
        $this->assertSame('Integer', $column['data_type']);
        $this->assertTrue($column['nullable']);
    }

    #[Test]
    public function it_adds_float_column(): void
    {
        $schema = TableSchema::create('products')
            ->float('price');

        $array = $schema->toArray();
        $column = $array['columns'][0];

        $this->assertSame('price', $column['name']);
        $this->assertSame('Float', $column['data_type']);
    }

    #[Test]
    public function it_adds_text_column(): void
    {
        $schema = TableSchema::create('posts')
            ->text('content', true);

        $array = $schema->toArray();
        $column = $array['columns'][0];

        $this->assertSame('content', $column['name']);
        $this->assertSame('Text', $column['data_type']);
        $this->assertTrue($column['nullable']);
    }

    #[Test]
    public function it_adds_string_alias_for_text(): void
    {
        $schema = TableSchema::create('users')
            ->string('name');

        $array = $schema->toArray();

        $this->assertSame('Text', $array['columns'][0]['data_type']);
    }

    #[Test]
    public function it_adds_boolean_column(): void
    {
        $schema = TableSchema::create('users')
            ->boolean('is_active');

        $array = $schema->toArray();

        $this->assertSame('Boolean', $array['columns'][0]['data_type']);
    }

    #[Test]
    public function it_adds_timestamp_column(): void
    {
        $schema = TableSchema::create('users')
            ->timestamp('created_at');

        $array = $schema->toArray();

        $this->assertSame('Timestamp', $array['columns'][0]['data_type']);
    }

    #[Test]
    public function it_adds_binary_column(): void
    {
        $schema = TableSchema::create('files')
            ->binary('content');

        $array = $schema->toArray();

        $this->assertSame('Binary', $array['columns'][0]['data_type']);
    }

    #[Test]
    public function it_adds_json_column(): void
    {
        $schema = TableSchema::create('settings')
            ->json('preferences');

        $array = $schema->toArray();

        $this->assertSame('Json', $array['columns'][0]['data_type']);
    }

    #[Test]
    public function it_adds_dna_column(): void
    {
        $schema = TableSchema::create('sequences')
            ->dna('sequence');

        $array = $schema->toArray();

        $this->assertSame('Dna', $array['columns'][0]['data_type']);
    }

    #[Test]
    public function it_adds_neural_column(): void
    {
        $schema = TableSchema::create('embeddings')
            ->neural('embedding');

        $array = $schema->toArray();

        $this->assertSame('Neural', $array['columns'][0]['data_type']);
    }

    #[Test]
    public function it_adds_quantum_column(): void
    {
        $schema = TableSchema::create('states')
            ->quantum('state');

        $array = $schema->toArray();

        $this->assertSame('Quantum', $array['columns'][0]['data_type']);
    }

    #[Test]
    public function it_adds_id_shortcut(): void
    {
        $schema = TableSchema::create('users')
            ->id();

        $array = $schema->toArray();

        $this->assertSame('id', $array['columns'][0]['name']);
        $this->assertTrue($array['columns'][0]['auto_increment']);
        $this->assertSame('PrimaryKey', $array['constraints'][0]['constraint_type']);
    }

    #[Test]
    public function it_adds_timestamps_shortcut(): void
    {
        $schema = TableSchema::create('users')
            ->timestamps();

        $array = $schema->toArray();

        $this->assertSame('created_at', $array['columns'][0]['name']);
        $this->assertSame('updated_at', $array['columns'][1]['name']);
    }

    #[Test]
    public function it_adds_unique_constraint(): void
    {
        $schema = TableSchema::create('users')
            ->text('email')
            ->unique('email');

        $array = $schema->toArray();
        $constraint = $array['constraints'][0];

        $this->assertSame('Unique', $constraint['constraint_type']);
        $this->assertSame(['email'], $constraint['columns']);
    }

    #[Test]
    public function it_adds_index(): void
    {
        $schema = TableSchema::create('users')
            ->text('name')
            ->index('name');

        $array = $schema->toArray();
        $constraint = $array['constraints'][0];

        $this->assertSame('Index', $constraint['constraint_type']);
        $this->assertSame(['name'], $constraint['columns']);
    }

    #[Test]
    public function it_adds_composite_primary_key(): void
    {
        $schema = TableSchema::create('order_items')
            ->integer('order_id')
            ->integer('product_id')
            ->primaryKey(['order_id', 'product_id']);

        $array = $schema->toArray();
        $constraint = $array['constraints'][0];

        $this->assertSame('PrimaryKey', $constraint['constraint_type']);
        $this->assertSame(['order_id', 'product_id'], $constraint['columns']);
    }

    #[Test]
    public function it_returns_table_name(): void
    {
        $schema = TableSchema::create('products');

        $this->assertSame('products', $schema->getName());
    }
}
