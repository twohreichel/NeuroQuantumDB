<?php

declare(strict_types=1);

namespace NeuroQuantum\Tests\Unit\Query;

use NeuroQuantum\Query\FilterOperator;
use NeuroQuantum\Query\UpdateBuilder;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

final class UpdateBuilderTest extends TestCase
{
    #[Test]
    public function it_builds_simple_update(): void
    {
        $builder = UpdateBuilder::table('users')
            ->set('name', 'John Doe')
            ->where('id', 1);

        $sql = $builder->toSql();

        $this->assertStringContainsString('UPDATE "users"', $sql);
        $this->assertStringContainsString('SET "name" = \'John Doe\'', $sql);
        $this->assertStringContainsString('WHERE "id" = 1', $sql);
    }

    #[Test]
    public function it_builds_update_with_multiple_columns(): void
    {
        $builder = UpdateBuilder::table('users')
            ->setMany(['name' => 'John', 'email' => 'john@example.com'])
            ->where('id', 1);

        $sql = $builder->toSql();

        $this->assertStringContainsString('"name" = \'John\'', $sql);
        $this->assertStringContainsString('"email" = \'john@example.com\'', $sql);
    }

    #[Test]
    public function it_builds_update_with_increment(): void
    {
        $builder = UpdateBuilder::table('products')
            ->increment('quantity', 5)
            ->where('id', 1);

        $sql = $builder->toSql();

        $this->assertStringContainsString('"quantity" = "quantity" + 5', $sql);
    }

    #[Test]
    public function it_builds_update_with_decrement(): void
    {
        $builder = UpdateBuilder::table('products')
            ->decrement('quantity', 3)
            ->where('id', 1);

        $sql = $builder->toSql();

        $this->assertStringContainsString('"quantity" = "quantity" - 3', $sql);
    }

    #[Test]
    public function it_builds_update_with_multiple_where_conditions(): void
    {
        $builder = UpdateBuilder::table('users')
            ->set('status', 'inactive')
            ->where('role', 'user')
            ->where('last_login', FilterOperator::LessThan, '2025-01-01');

        $sql = $builder->toSql();

        $this->assertStringContainsString('"role" = \'user\'', $sql);
        $this->assertStringContainsString('"last_login" < \'2025-01-01\'', $sql);
    }

    #[Test]
    public function it_handles_null_values(): void
    {
        $builder = UpdateBuilder::table('users')
            ->set('deleted_at', null)
            ->where('id', 1);

        $sql = $builder->toSql();

        $this->assertStringContainsString('"deleted_at" = NULL', $sql);
    }

    #[Test]
    public function it_handles_boolean_values(): void
    {
        $builder = UpdateBuilder::table('users')
            ->set('is_active', true)
            ->where('id', 1);

        $sql = $builder->toSql();

        $this->assertStringContainsString('"is_active" = TRUE', $sql);
    }

    #[Test]
    public function it_converts_to_array_for_api(): void
    {
        $builder = UpdateBuilder::table('users')
            ->set('name', 'John')
            ->where('id', 1);

        $array = $builder->toArray();

        $this->assertSame('users', $array['table_name']);
        $this->assertArrayHasKey('updates', $array);
        $this->assertArrayHasKey('filters', $array);
    }

    #[Test]
    public function it_returns_table_name(): void
    {
        $builder = UpdateBuilder::table('products');

        $this->assertSame('products', $builder->getTable());
    }

    #[Test]
    public function it_returns_empty_sql_when_no_updates(): void
    {
        $builder = UpdateBuilder::table('users');

        $this->assertSame('', $builder->toSql());
    }

    #[Test]
    public function it_escapes_single_quotes(): void
    {
        $builder = UpdateBuilder::table('users')
            ->set('name', "O'Brien")
            ->where('id', 1);

        $sql = $builder->toSql();

        $this->assertStringContainsString("'O''Brien'", $sql);
    }
}
