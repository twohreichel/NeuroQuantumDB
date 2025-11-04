// NeuroQuantumDB Load Testing Script
// Uses k6 for performance and stress testing
// Run: k6 run load-test.js

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const queryDuration = new Trend('query_duration');
const queryCount = new Counter('query_count');

// Test configuration
export const options = {
  stages: [
    { duration: '30s', target: 10 },   // Ramp up to 10 users
    { duration: '1m', target: 50 },    // Ramp up to 50 users
    { duration: '2m', target: 100 },   // Peak at 100 users
    { duration: '1m', target: 50 },    // Ramp down to 50 users
    { duration: '30s', target: 0 },    // Ramp down to 0 users
  ],
  thresholds: {
    http_req_duration: ['p(95)<500'],  // 95% of requests < 500ms
    errors: ['rate<0.05'],              // Error rate < 5%
    query_duration: ['p(99)<1000'],     // 99% of queries < 1s
  },
};

// Base configuration
const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';
const API_KEY = __ENV.API_KEY || 'test-api-key';

// Headers
const headers = {
  'Content-Type': 'application/json',
  'Authorization': `Bearer ${API_KEY}`,
};

// Test scenarios
export default function () {
  // Scenario 1: Health check
  testHealthCheck();

  // Scenario 2: Query execution
  testQuery();

  // Scenario 3: DNA compression
  testDNACompression();

  // Scenario 4: Quantum search
  testQuantumSearch();

  // Scenario 5: Transaction
  testTransaction();

  sleep(1); // Think time between iterations
}

function testHealthCheck() {
  const res = http.get(`${BASE_URL}/health`);

  check(res, {
    'health check status is 200': (r) => r.status === 200,
    'health check response time < 100ms': (r) => r.timings.duration < 100,
  });

  errorRate.add(res.status !== 200);
}

function testQuery() {
  const payload = JSON.stringify({
    query: 'SELECT * FROM users WHERE id = ?',
    params: [Math.floor(Math.random() * 1000)],
  });

  const startTime = Date.now();
  const res = http.post(`${BASE_URL}/api/v1/query`, payload, { headers });
  const duration = Date.now() - startTime;

  check(res, {
    'query status is 200': (r) => r.status === 200,
    'query response time < 500ms': (r) => r.timings.duration < 500,
    'query returns data': (r) => {
      try {
        const body = JSON.parse(r.body);
        return body.data !== undefined;
      } catch {
        return false;
      }
    },
  });

  queryDuration.add(duration);
  queryCount.add(1);
  errorRate.add(res.status !== 200);
}

function testDNACompression() {
  const dnaSequence = 'ATCG'.repeat(100); // 400 base pairs

  const payload = JSON.stringify({
    operation: 'compress',
    data: dnaSequence,
    algorithm: 'dna_compression',
  });

  const res = http.post(`${BASE_URL}/api/v1/compress`, payload, { headers });

  check(res, {
    'compression status is 200': (r) => r.status === 200,
    'compression response time < 200ms': (r) => r.timings.duration < 200,
    'compression ratio > 2': (r) => {
      try {
        const body = JSON.parse(r.body);
        return body.compression_ratio > 2;
      } catch {
        return false;
      }
    },
  });

  errorRate.add(res.status !== 200);
}

function testQuantumSearch() {
  const payload = JSON.stringify({
    algorithm: 'grover',
    target: Math.floor(Math.random() * 256),
    qubits: 8,
  });

  const res = http.post(`${BASE_URL}/api/v1/quantum/search`, payload, { headers });

  check(res, {
    'quantum search status is 200': (r) => r.status === 200,
    'quantum search response time < 1s': (r) => r.timings.duration < 1000,
  });

  errorRate.add(res.status !== 200);
}

function testTransaction() {
  const payload = JSON.stringify({
    operations: [
      { type: 'insert', table: 'users', data: { name: `user_${__VU}_${__ITER}` } },
      { type: 'update', table: 'users', id: __VU, data: { updated: true } },
    ],
  });

  const res = http.post(`${BASE_URL}/api/v1/transaction`, payload, { headers });

  check(res, {
    'transaction status is 200': (r) => r.status === 200,
    'transaction response time < 1s': (r) => r.timings.duration < 1000,
    'transaction committed': (r) => {
      try {
        const body = JSON.parse(r.body);
        return body.status === 'committed';
      } catch {
        return false;
      }
    },
  });

  errorRate.add(res.status !== 200);
}

// Teardown function
export function teardown(data) {
  console.log('Load test completed!');
  console.log(`Total queries: ${queryCount.value}`);
  console.log(`Error rate: ${(errorRate.value * 100).toFixed(2)}%`);
}

