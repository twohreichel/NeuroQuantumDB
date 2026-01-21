<?php

declare(strict_types=1);

namespace NeuroQuantum\Http;

use GuzzleHttp\Client;
use GuzzleHttp\ClientInterface;
use GuzzleHttp\Exception\ConnectException;
use GuzzleHttp\Exception\GuzzleException;
use GuzzleHttp\Exception\RequestException;
use GuzzleHttp\HandlerStack;
use GuzzleHttp\Middleware;
use GuzzleHttp\Psr7\Request;
use NeuroQuantum\Contract\ConfigInterface;
use NeuroQuantum\Contract\ConnectionInterface;
use NeuroQuantum\Exception\ConnectionException;
use NeuroQuantum\Exception\NeuroQuantumException;
use Psr\Http\Message\ResponseInterface;
use Psr\Log\LoggerInterface;
use Psr\Log\NullLogger;

/**
 * HTTP-based connection to NeuroQuantumDB.
 */
final class HttpConnection implements ConnectionInterface
{
    private ClientInterface $client;
    private bool $connected = false;

    public function __construct(
        private readonly ConfigInterface $config,
        private readonly LoggerInterface $logger = new NullLogger(),
        ?ClientInterface $client = null,
    ) {
        $this->client = $client ?? $this->createClient();
    }

    public function get(string $endpoint, array $query = []): array
    {
        return $this->request('GET', $endpoint, ['query' => $query]);
    }

    public function post(string $endpoint, array $data = []): array
    {
        return $this->request('POST', $endpoint, ['json' => $data]);
    }

    public function put(string $endpoint, array $data = []): array
    {
        return $this->request('PUT', $endpoint, ['json' => $data]);
    }

    public function delete(string $endpoint, array $data = []): array
    {
        $options = $data !== [] ? ['json' => $data] : [];
        return $this->request('DELETE', $endpoint, $options);
    }

    public function isConnected(): bool
    {
        return $this->connected;
    }

    public function getBaseUrl(): string
    {
        return $this->config->getBaseUrl();
    }

    /**
     * Test the connection by calling the health endpoint.
     *
     * @throws ConnectionException When connection fails
     */
    public function connect(): void
    {
        try {
            // Health endpoint is at /health, not under /api/v1
            $healthUrl = sprintf(
                '%s://%s:%d/health',
                $this->config->useSsl() ? 'https' : 'http',
                $this->config->getHost(),
                $this->config->getPort()
            );

            $response = $this->client->request('GET', $healthUrl);
            $statusCode = $response->getStatusCode();

            if ($statusCode !== 200) {
                throw ConnectionException::failed(
                    $this->config->getHost(),
                    $this->config->getPort(),
                    'Health check returned status ' . $statusCode
                );
            }

            $this->connected = true;
            $this->logger->info('Connected to NeuroQuantumDB', [
                'host' => $this->config->getHost(),
                'port' => $this->config->getPort(),
            ]);
        } catch (ConnectException $e) {
            throw ConnectionException::failed(
                $this->config->getHost(),
                $this->config->getPort(),
                $e->getMessage()
            );
        } catch (GuzzleException $e) {
            throw ConnectionException::failed(
                $this->config->getHost(),
                $this->config->getPort(),
                $e->getMessage()
            );
        }
    }

    /**
     * Send HTTP request and parse response.
     *
     * @param array<string, mixed> $options Guzzle request options
     * @return array<string, mixed> Parsed response data
     * @throws NeuroQuantumException When request fails
     */
    private function request(string $method, string $endpoint, array $options = []): array
    {
        $url = $this->config->getBaseUrl() . $endpoint;

        $this->logger->debug('Sending request', [
            'method' => $method,
            'url' => $url,
        ]);

        try {
            $response = $this->client->request($method, $url, $options);
            $statusCode = $response->getStatusCode();
            $body = $response->getBody()->getContents();
            $data = json_decode($body, true, 512, JSON_THROW_ON_ERROR);

            $apiResponse = ApiResponse::fromArray($data, $statusCode);

            if (!$apiResponse->isSuccess()) {
                $headers = $this->extractRateLimitHeaders($response);
                throw ExceptionFactory::fromResponse($apiResponse, $headers);
            }

            $this->connected = true;

            return $apiResponse->data ?? [];
        } catch (ConnectException $e) {
            $this->connected = false;
            throw ConnectionException::failed(
                $this->config->getHost(),
                $this->config->getPort(),
                $e->getMessage()
            );
        } catch (RequestException $e) {
            return $this->handleRequestException($e);
        } catch (\JsonException $e) {
            throw new NeuroQuantumException('Invalid JSON response: ' . $e->getMessage());
        } catch (GuzzleException $e) {
            throw new NeuroQuantumException('HTTP request failed: ' . $e->getMessage());
        }
    }

    /**
     * Handle Guzzle request exception.
     *
     * @return array<string, mixed>
     * @throws NeuroQuantumException
     */
    private function handleRequestException(RequestException $e): array
    {
        $response = $e->getResponse();
        if ($response === null) {
            throw new NeuroQuantumException('Request failed: ' . $e->getMessage());
        }

        $statusCode = $response->getStatusCode();
        $body = $response->getBody()->getContents();

        try {
            $data = json_decode($body, true, 512, JSON_THROW_ON_ERROR);
            $apiResponse = ApiResponse::fromArray($data, $statusCode);
            $headers = $this->extractRateLimitHeaders($response);
            throw ExceptionFactory::fromResponse($apiResponse, $headers);
        } catch (\JsonException) {
            throw new NeuroQuantumException(
                'Request failed with status ' . $statusCode . ': ' . $body,
                $statusCode
            );
        }
    }

    /**
     * Extract rate limit headers from response.
     *
     * @return array<string, string|int|null>
     */
    private function extractRateLimitHeaders(ResponseInterface $response): array
    {
        return [
            'X-RateLimit-Limit' => $response->getHeaderLine('X-RateLimit-Limit') ?: null,
            'X-RateLimit-Remaining' => $response->getHeaderLine('X-RateLimit-Remaining') ?: null,
            'X-RateLimit-Reset' => $response->getHeaderLine('X-RateLimit-Reset') ?: null,
        ];
    }

    /**
     * Create configured Guzzle client.
     */
    private function createClient(): Client
    {
        $stack = HandlerStack::create();

        if ($this->config->isRetryEnabled()) {
            $stack->push($this->createRetryMiddleware());
        }

        return new Client([
            'handler' => $stack,
            'timeout' => $this->config->getTimeout(),
            'connect_timeout' => $this->config->getTimeout(),
            'headers' => [
                'X-API-Key' => $this->config->getApiKey(),
                'Content-Type' => 'application/json',
                'Accept' => 'application/json',
                'User-Agent' => 'NeuroQuantumDB-PHP/1.0',
            ],
            'http_errors' => false,
        ]);
    }

    /**
     * Create retry middleware.
     */
    private function createRetryMiddleware(): callable
    {
        $maxAttempts = $this->config->getMaxRetryAttempts();
        $logger = $this->logger;

        return Middleware::retry(
            function (int $retries, Request $request, ?ResponseInterface $response = null, ?\Throwable $exception = null) use ($maxAttempts, $logger): bool {
                if ($retries >= $maxAttempts) {
                    return false;
                }

                // Retry on connection errors
                if ($exception instanceof ConnectException) {
                    $logger->warning('Retrying request due to connection error', [
                        'attempt' => $retries + 1,
                        'max_attempts' => $maxAttempts,
                    ]);
                    return true;
                }

                // Retry on 5xx errors and 429
                if ($response !== null) {
                    $statusCode = $response->getStatusCode();
                    if ($statusCode >= 500 || $statusCode === 429) {
                        $logger->warning('Retrying request due to server error', [
                            'status_code' => $statusCode,
                            'attempt' => $retries + 1,
                            'max_attempts' => $maxAttempts,
                        ]);
                        return true;
                    }
                }

                return false;
            },
            function (int $retries): int {
                // Exponential backoff: 1s, 2s, 4s, ...
                return (int) (1000 * pow(2, $retries));
            }
        );
    }
}
