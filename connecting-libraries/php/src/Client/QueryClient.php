<?php

declare(strict_types=1);

namespace NeuroQuantum\Client;

use NeuroQuantum\Exception\QueryException;
use NeuroQuantum\Query\DeleteBuilder;
use NeuroQuantum\Query\InsertBuilder;
use NeuroQuantum\Query\SelectBuilder;
use NeuroQuantum\Query\UpdateBuilder;

/**
 * Client for executing raw SQL queries via the /query endpoint.
 */
final class QueryClient extends AbstractClient
{
    /**
     * Execute a raw QSQL query.
     *
     * @return QueryResult The query result
     * @throws QueryException When query execution fails
     */
    public function execute(string $sql): QueryResult
    {
        $response = $this->connection->post('/query', ['query' => $sql]);
        return QueryResult::fromArray($response);
    }

    /**
     * Execute a SelectBuilder query.
     *
     * @return array<int, array<string, mixed>> The query rows
     */
    public function select(SelectBuilder $builder): array
    {
        $result = $this->execute($builder->toSql());
        return $result->rows;
    }

    /**
     * Execute an InsertBuilder query.
     *
     * @return int Number of rows inserted
     */
    public function insert(InsertBuilder $builder): int
    {
        $result = $this->execute($builder->toSql());
        return $result->rowsAffected;
    }

    /**
     * Execute an UpdateBuilder query.
     *
     * @return int Number of rows updated
     */
    public function update(UpdateBuilder $builder): int
    {
        $result = $this->execute($builder->toSql());
        return $result->rowsAffected;
    }

    /**
     * Execute a DeleteBuilder query.
     *
     * @return int Number of rows deleted
     */
    public function delete(DeleteBuilder $builder): int
    {
        $result = $this->execute($builder->toSql());
        return $result->rowsAffected;
    }

    /**
     * Begin a transaction.
     */
    public function beginTransaction(): void
    {
        $this->execute('BEGIN TRANSACTION');
    }

    /**
     * Commit the current transaction.
     */
    public function commit(): void
    {
        $this->execute('COMMIT');
    }

    /**
     * Rollback the current transaction.
     */
    public function rollback(): void
    {
        $this->execute('ROLLBACK');
    }

    /**
     * Execute callback within a transaction.
     *
     * @template T
     * @param callable(): T $callback
     * @return T The callback result
     * @throws \Throwable Re-throws any exception after rollback
     */
    public function transaction(callable $callback): mixed
    {
        $this->beginTransaction();

        try {
            $result = $callback();
            $this->commit();
            return $result;
        } catch (\Throwable $e) {
            $this->rollback();
            throw $e;
        }
    }
}
