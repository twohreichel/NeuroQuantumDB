<?php

declare(strict_types=1);

namespace NeuroQuantum\Tests\Unit\Query;

use NeuroQuantum\Query\FilterOperator;
use NeuroQuantum\Query\SelectBuilder;
use NeuroQuantum\Query\SortDirection;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

final class SelectBuilderTest extends TestCase
{
    #[Test]
    public function it_builds_simple_select(): void
    {
        $builder = SelectBuilder::from('users');
        $sql = $builder->toSql();

        $this->assertSame('SELECT * FROM "users"', $sql);
    }

    #[Test]
    public function it_builds_select_with_columns(): void
    {
        $builder = SelectBuilder::from('users')
            ->select(['id', 'name', 'email']);

        $sql = $builder->toSql();

        $this->assertSame('SELECT "id", "name", "email" FROM "users"', $sql);
    }

    #[Test]
    public function it_builds_select_with_where_clause(): void
    {
        $builder = SelectBuilder::from('users')
            ->where('status', 'active');

        $sql = $builder->toSql();

        $this->assertStringContainsString('WHERE "status" = \'active\'', $sql);
    }

    #[Test]
    public function it_builds_select_with_comparison_operators(): void
    {
        $builder = SelectBuilder::from('users')
            ->where('age', '>', 18)
            ->where('score', '<=', 100);

        $sql = $builder->toSql();

        $this->assertStringContainsString('"age" > 18', $sql);
        $this->assertStringContainsString('"score" <= 100', $sql);
    }

    #[Test]
    public function it_builds_select_with_filter_operator_enum(): void
    {
        $builder = SelectBuilder::from('users')
            ->where('age', FilterOperator::GreaterThan, 21);

        $sql = $builder->toSql();

        $this->assertStringContainsString('"age" > 21', $sql);
    }

    #[Test]
    public function it_builds_select_with_or_where(): void
    {
        $builder = SelectBuilder::from('users')
            ->where('role', 'admin')
            ->orWhere('role', 'superuser');

        $sql = $builder->toSql();

        $this->assertStringContainsString('"role" = \'admin\'', $sql);
        $this->assertStringContainsString('OR "role" = \'superuser\'', $sql);
    }

    #[Test]
    public function it_builds_select_with_where_in(): void
    {
        $builder = SelectBuilder::from('users')
            ->whereIn('status', ['active', 'pending']);

        $sql = $builder->toSql();

        $this->assertStringContainsString('"status" IN (\'active\', \'pending\')', $sql);
    }

    #[Test]
    public function it_builds_select_with_where_null(): void
    {
        $builder = SelectBuilder::from('users')
            ->whereNull('deleted_at');

        $sql = $builder->toSql();

        $this->assertStringContainsString('"deleted_at" IS NULL', $sql);
    }

    #[Test]
    public function it_builds_select_with_where_not_null(): void
    {
        $builder = SelectBuilder::from('users')
            ->whereNotNull('email_verified_at');

        $sql = $builder->toSql();

        $this->assertStringContainsString('"email_verified_at" IS NOT NULL', $sql);
    }

    #[Test]
    public function it_builds_select_with_like(): void
    {
        $builder = SelectBuilder::from('users')
            ->whereLike('name', 'John%');

        $sql = $builder->toSql();

        $this->assertStringContainsString('"name" LIKE \'John%\'', $sql);
    }

    #[Test]
    public function it_builds_select_with_contains(): void
    {
        $builder = SelectBuilder::from('users')
            ->whereContains('email', 'gmail');

        $sql = $builder->toSql();

        $this->assertStringContainsString('"email" LIKE \'%gmail%\'', $sql);
    }

    #[Test]
    public function it_builds_select_with_order_by(): void
    {
        $builder = SelectBuilder::from('users')
            ->orderBy('name', SortDirection::Asc)
            ->orderByDesc('created_at');

        $sql = $builder->toSql();

        $this->assertStringContainsString('ORDER BY "name" ASC, "created_at" DESC', $sql);
    }

    #[Test]
    public function it_builds_select_with_limit_and_offset(): void
    {
        $builder = SelectBuilder::from('users')
            ->limit(10)
            ->offset(20);

        $sql = $builder->toSql();

        $this->assertStringContainsString('LIMIT 10', $sql);
        $this->assertStringContainsString('OFFSET 20', $sql);
    }

    #[Test]
    public function it_builds_select_with_pagination(): void
    {
        $builder = SelectBuilder::from('users')
            ->paginate(3, 15);

        $sql = $builder->toSql();

        $this->assertStringContainsString('LIMIT 15', $sql);
        $this->assertStringContainsString('OFFSET 30', $sql);
    }

    #[Test]
    public function it_builds_select_with_group_by(): void
    {
        $builder = SelectBuilder::from('orders')
            ->select(['customer_id', 'COUNT(*) as count'])
            ->groupBy('customer_id');

        $sql = $builder->toSql();

        $this->assertStringContainsString('GROUP BY "customer_id"', $sql);
    }

    #[Test]
    public function it_builds_select_with_having(): void
    {
        $builder = SelectBuilder::from('orders')
            ->select(['customer_id', 'SUM(amount) as total'])
            ->groupBy('customer_id')
            ->having('total', '>', 1000);

        $sql = $builder->toSql();

        $this->assertStringContainsString('HAVING "total" > 1000', $sql);
    }

    #[Test]
    public function it_converts_to_array_for_api(): void
    {
        $builder = SelectBuilder::from('users')
            ->select(['id', 'name'])
            ->where('status', 'active')
            ->orderBy('name')
            ->limit(10)
            ->offset(5);

        $array = $builder->toArray();

        $this->assertSame('users', $array['table_name']);
        $this->assertSame(['id', 'name'], $array['columns']);
        $this->assertArrayHasKey('filters', $array);
        $this->assertSame(10, $array['limit']);
        $this->assertSame(5, $array['offset']);
    }

    #[Test]
    public function it_returns_empty_bindings(): void
    {
        $builder = SelectBuilder::from('users');
        $bindings = $builder->getBindings();

        $this->assertIsArray($bindings);
    }

    #[Test]
    public function it_returns_table_name(): void
    {
        $builder = SelectBuilder::from('products');

        $this->assertSame('products', $builder->getTable());
    }

    #[Test]
    public function it_escapes_single_quotes_in_values(): void
    {
        $builder = SelectBuilder::from('users')
            ->where('name', "O'Brien");

        $sql = $builder->toSql();

        $this->assertStringContainsString("'O''Brien'", $sql);
    }

    #[Test]
    public function it_handles_boolean_values(): void
    {
        $builder = SelectBuilder::from('users')
            ->where('is_active', true)
            ->where('is_deleted', false);

        $sql = $builder->toSql();

        $this->assertStringContainsString('"is_active" = TRUE', $sql);
        $this->assertStringContainsString('"is_deleted" = FALSE', $sql);
    }

    #[Test]
    public function it_handles_numeric_values(): void
    {
        $builder = SelectBuilder::from('products')
            ->where('price', 19.99)
            ->where('quantity', 100);

        $sql = $builder->toSql();

        $this->assertStringContainsString('"price" = 19.99', $sql);
        $this->assertStringContainsString('"quantity" = 100', $sql);
    }
}
