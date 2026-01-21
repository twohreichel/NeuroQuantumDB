<?php

declare(strict_types=1);

namespace NeuroQuantum\Client;

/**
 * Client for DNA compression operations.
 */
final class DnaClient extends AbstractClient
{
    public const ALGORITHM_KMER = 'KmerBased';
    public const ALGORITHM_NEURAL = 'NeuralCompression';
    public const ALGORITHM_QUANTUM = 'QuantumInspired';
    public const ALGORITHM_HYBRID = 'HybridApproach';

    /**
     * Compress DNA sequences.
     *
     * @param array<int, string> $sequences DNA sequences to compress
     * @param string $algorithm Compression algorithm
     * @param int $compressionLevel Compression level (1-9)
     * @return array<string, mixed> Compression result
     */
    public function compress(
        array $sequences,
        string $algorithm = self::ALGORITHM_KMER,
        int $compressionLevel = 5,
    ): array {
        return $this->connection->post('/dna/compress', [
            'sequences' => $sequences,
            'algorithm' => $algorithm,
            'compression_level' => $compressionLevel,
        ]);
    }

    /**
     * Decompress DNA data.
     *
     * @param string $compressedData Base64 encoded compressed data
     * @param string $algorithm Algorithm used for compression
     * @return array<int, string> Decompressed DNA sequences
     */
    public function decompress(string $compressedData, string $algorithm = self::ALGORITHM_KMER): array
    {
        $response = $this->connection->post('/dna/decompress', [
            'compressed_data' => $compressedData,
            'algorithm' => $algorithm,
        ]);

        return $response['sequences'] ?? [];
    }
}
