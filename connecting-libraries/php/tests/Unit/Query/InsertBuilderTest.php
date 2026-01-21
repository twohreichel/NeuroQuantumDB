<?php

declare(strict_types=1);

namespace NeuroQuantum\Tests\Unit\Query;

use NeuroQuantum\Query\InsertBuilder;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

final class InsertBuilderTest extends TestCase
{
    #[Test]
    public function it_builds_simple_insert(): void
    {
        $builder = InsertBuilder::into('users')
            ->values(['name' => 'John', 'email' => 'john@example.com']);

        $sql = $builder->toSql();

        $this->assertStringContainsString('INSERT INTO "users"', $sql);
        $this->assertStringContainsString('"name"', $sql);
        $this->assertStringContainsString('"email"', $sql);
        $this->assertStringContainsString("'John'", $sql);
        $this->assertStringContainsString("'john@example.com'", $sql);
    }

    #[Test]
    public function it_builds_multi_row_insert(): void
    {
        $builder = InsertBuilder::into('users')
            ->insert([
                ['name' => 'John', 'email' => 'john@example.com'],
                ['name' => 'Jane', 'email' => 'jane@example.com'],
            ]);

        $sql = $builder->toSql();

        $this->assertStringContainsString("'John'", $sql);
        $this->assertStringContainsString("'Jane'", $sql);
    }

    #[Test]
    public function it_handles_null_values(): void
    {
        $builder = InsertBuilder::into('users')
            ->values(['name' => 'John', 'bio' => null]);

        $sql = $builder->toSql();

        $this->assertStringContainsString('NULL', $sql);
    }

    #[Test]
    public function it_handles_boolean_values(): void
    {
        $builder = InsertBuilder::into('users')
            ->values(['name' => 'John', 'is_active' => true, 'is_deleted' => false]);

        $sql = $builder->toSql();

        $this->assertStringContainsString('TRUE', $sql);
        $this->assertStringContainsString('FALSE', $sql);
    }

    #[Test]
    public function it_handles_numeric_values(): void
    {
        $builder = InsertBuilder::into('products')
            ->values(['name' => 'Widget', 'price' => 19.99, 'quantity' => 100]);

        $sql = $builder->toSql();

        $this->assertStringContainsString('19.99', $sql);
        $this->assertStringContainsString('100', $sql);
    }

    #[Test]
    public function it_handles_json_array_values(): void
    {
        $builder = InsertBuilder::into('users')
            ->values(['name' => 'John', 'settings' => ['theme' => 'dark']]);

        $sql = $builder->toSql();

        $this->assertStringContainsString('{"theme":"dark"}', $sql);
    }

    #[Test]
    public function it_sets_batch_size(): void
    {
        $builder = InsertBuilder::into('users')
            ->values(['name' => 'John'])
            ->batchSize(500);

        $array = $builder->toArray();

        $this->assertSame(500, $array['batch_size']);
    }

    #[Test]
    public function it_sets_on_conflict_strategy(): void
    {
        $builder = InsertBuilder::into('users')
            ->values(['name' => 'John'])
            ->onConflict('Ignore');

        $array = $builder->toArray();

        $this->assertSame('Ignore', $array['on_conflict']);
    }

    #[Test]
    public function it_uses_or_ignore_shortcut(): void
    {
        $builder = InsertBuilder::into('users')
            ->values(['name' => 'John'])
            ->orIgnore();

        $array = $builder->toArray();

        $this->assertSame('Ignore', $array['on_conflict']);
    }

    #[Test]
    public function it_uses_or_replace_shortcut(): void
    {
        $builder = InsertBuilder::into('users')
            ->values(['name' => 'John'])
            ->orReplace();

        $array = $builder->toArray();

        $this->assertSame('Replace', $array['on_conflict']);
    }

    #[Test]
    public function it_converts_to_array_for_api(): void
    {
        $builder = InsertBuilder::into('users')
            ->insert([
                ['name' => 'John', 'email' => 'john@example.com'],
            ])
            ->batchSize(1000)
            ->onConflict('Error');

        $array = $builder->toArray();

        $this->assertSame('users', $array['table_name']);
        $this->assertCount(1, $array['records']);
        $this->assertSame(1000, $array['batch_size']);
        $this->assertSame('Error', $array['on_conflict']);
    }

    #[Test]
    public function it_returns_table_name(): void
    {
        $builder = InsertBuilder::into('products');

        $this->assertSame('products', $builder->getTable());
    }

    #[Test]
    public function it_returns_records(): void
    {
        $records = [['name' => 'John'], ['name' => 'Jane']];
        $builder = InsertBuilder::into('users')->insert($records);

        $this->assertSame($records, $builder->getRecords());
    }

    #[Test]
    public function it_escapes_single_quotes(): void
    {
        $builder = InsertBuilder::into('users')
            ->values(['name' => "O'Brien"]);

        $sql = $builder->toSql();

        $this->assertStringContainsString("'O''Brien'", $sql);
    }

    #[Test]
    public function it_returns_empty_sql_when_no_records(): void
    {
        $builder = InsertBuilder::into('users');

        $this->assertSame('', $builder->toSql());
    }
}
