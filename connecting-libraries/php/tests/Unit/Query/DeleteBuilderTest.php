<?php

declare(strict_types=1);

namespace NeuroQuantum\Tests\Unit\Query;

use NeuroQuantum\Query\DeleteBuilder;
use NeuroQuantum\Query\FilterOperator;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

final class DeleteBuilderTest extends TestCase
{
    #[Test]
    public function it_builds_delete_all(): void
    {
        $builder = DeleteBuilder::from('users');
        $sql = $builder->toSql();

        $this->assertSame('DELETE FROM "users"', $sql);
    }

    #[Test]
    public function it_builds_delete_with_where(): void
    {
        $builder = DeleteBuilder::from('users')
            ->where('id', 1);

        $sql = $builder->toSql();

        $this->assertStringContainsString('DELETE FROM "users"', $sql);
        $this->assertStringContainsString('WHERE "id" = 1', $sql);
    }

    #[Test]
    public function it_builds_delete_with_multiple_conditions(): void
    {
        $builder = DeleteBuilder::from('users')
            ->where('status', 'inactive')
            ->where('created_at', FilterOperator::LessThan, '2024-01-01');

        $sql = $builder->toSql();

        $this->assertStringContainsString('"status" = \'inactive\'', $sql);
        $this->assertStringContainsString('"created_at" < \'2024-01-01\'', $sql);
    }

    #[Test]
    public function it_builds_delete_with_where_in(): void
    {
        $builder = DeleteBuilder::from('users')
            ->whereIn('id', [1, 2, 3]);

        $sql = $builder->toSql();

        $this->assertStringContainsString('"id" IN (1, 2, 3)', $sql);
    }

    #[Test]
    public function it_builds_delete_with_where_null(): void
    {
        $builder = DeleteBuilder::from('sessions')
            ->whereNull('user_id');

        $sql = $builder->toSql();

        $this->assertStringContainsString('"user_id" IS NULL', $sql);
    }

    #[Test]
    public function it_converts_to_array_for_api(): void
    {
        $builder = DeleteBuilder::from('users')
            ->where('id', 1);

        $array = $builder->toArray();

        $this->assertSame('users', $array['table_name']);
        $this->assertArrayHasKey('filters', $array);
    }

    #[Test]
    public function it_returns_table_name(): void
    {
        $builder = DeleteBuilder::from('products');

        $this->assertSame('products', $builder->getTable());
    }

    #[Test]
    public function it_returns_bindings(): void
    {
        $builder = DeleteBuilder::from('users');
        $bindings = $builder->getBindings();

        $this->assertIsArray($bindings);
    }
}
