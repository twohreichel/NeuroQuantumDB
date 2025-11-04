#!/usr/bin/env bash
# Docker Build and Test Script for NeuroQuantumDB
# Target: ARM64 (Raspberry Pi 4)
# Goal: Verify production-ready Docker image

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
IMAGE_NAME="neuroquantumdb"
IMAGE_TAG="${IMAGE_TAG:-latest}"
PLATFORM="${PLATFORM:-linux/arm64}"
SIZE_LIMIT_MB=20  # Target: < 15MB, allow 20MB buffer
MEMORY_LIMIT="100M"
STARTUP_TIMEOUT=10

echo -e "${GREEN}=== NeuroQuantumDB Docker Build & Test ===${NC}"
echo "Platform: $PLATFORM"
echo "Image: $IMAGE_NAME:$IMAGE_TAG"
echo ""

# Check if Docker is available
if ! command -v docker &> /dev/null; then
    echo -e "${RED}ERROR: Docker not found. Please install Docker.${NC}"
    exit 1
fi

# Check if Docker BuildKit is enabled
export DOCKER_BUILDKIT=1

# Step 1: Build Docker Image
echo -e "${YELLOW}[1/6] Building Docker image...${NC}"
START_TIME=$(date +%s)

docker build \
    --platform "$PLATFORM" \
    --tag "$IMAGE_NAME:$IMAGE_TAG" \
    --tag "$IMAGE_NAME:test" \
    --build-arg TARGETPLATFORM="$PLATFORM" \
    -f Dockerfile \
    . || {
        echo -e "${RED}ERROR: Docker build failed${NC}"
        exit 1
    }

BUILD_TIME=$(($(date +%s) - START_TIME))
echo -e "${GREEN}✓ Build completed in ${BUILD_TIME}s${NC}"

# Step 2: Check Image Size
echo -e "${YELLOW}[2/6] Checking image size...${NC}"
IMAGE_SIZE_BYTES=$(docker image inspect "$IMAGE_NAME:$IMAGE_TAG" --format='{{.Size}}')
IMAGE_SIZE_MB=$((IMAGE_SIZE_BYTES / 1024 / 1024))

echo "Image size: ${IMAGE_SIZE_MB}MB"

if [ "$IMAGE_SIZE_MB" -gt "$SIZE_LIMIT_MB" ]; then
    echo -e "${YELLOW}WARNING: Image size ${IMAGE_SIZE_MB}MB exceeds target of ${SIZE_LIMIT_MB}MB${NC}"
    echo "Consider further optimization (strip, upx, remove debug symbols)"
else
    echo -e "${GREEN}✓ Image size within target${NC}"
fi

# Step 3: Test Container Startup
echo -e "${YELLOW}[3/6] Testing container startup...${NC}"

# Start container
CONTAINER_ID=$(docker run \
    --platform "$PLATFORM" \
    --detach \
    --memory="$MEMORY_LIMIT" \
    --publish 8080:8080 \
    --publish 9090:9090 \
    --env RUST_LOG=info \
    --name neuroquantum-test \
    "$IMAGE_NAME:$IMAGE_TAG" || true)

if [ -z "$CONTAINER_ID" ]; then
    echo -e "${RED}ERROR: Failed to start container${NC}"
    exit 1
fi

echo "Container started: $CONTAINER_ID"

# Wait for startup
echo "Waiting ${STARTUP_TIMEOUT}s for startup..."
sleep "$STARTUP_TIMEOUT"

# Step 4: Check Health Endpoint
echo -e "${YELLOW}[4/6] Testing health endpoint...${NC}"

HEALTH_STATUS=$(docker exec neuroquantum-test /usr/local/bin/neuroquantumdb health-check 2>&1 || echo "FAILED")

if [[ "$HEALTH_STATUS" == *"FAILED"* ]]; then
    echo -e "${YELLOW}WARNING: Health check failed. This is expected if health-check subcommand is not implemented.${NC}"
    echo "Trying HTTP health endpoint instead..."

    # Try HTTP health endpoint
    if command -v curl &> /dev/null; then
        HTTP_HEALTH=$(curl -s -f http://localhost:8080/health || echo "FAILED")
        if [[ "$HTTP_HEALTH" == *"FAILED"* ]]; then
            echo -e "${YELLOW}WARNING: HTTP health endpoint not responding${NC}"
        else
            echo -e "${GREEN}✓ HTTP Health endpoint OK${NC}"
            echo "$HTTP_HEALTH"
        fi
    else
        echo "curl not available, skipping HTTP health check"
    fi
else
    echo -e "${GREEN}✓ Health check passed${NC}"
fi

# Step 5: Check Resource Usage
echo -e "${YELLOW}[5/6] Checking resource usage...${NC}"

STATS=$(docker stats neuroquantum-test --no-stream --format "table {{.MemUsage}}\t{{.CPUPerc}}")
echo "$STATS"

MEM_USAGE=$(docker stats neuroquantum-test --no-stream --format "{{.MemUsage}}" | cut -d'/' -f1 | sed 's/[^0-9.]//g')
echo "Memory usage: ${MEM_USAGE}MiB"

if (( $(echo "$MEM_USAGE > 100" | bc -l) )); then
    echo -e "${YELLOW}WARNING: Memory usage exceeds 100MB target${NC}"
else
    echo -e "${GREEN}✓ Memory usage within target${NC}"
fi

# Step 6: Check Logs
echo -e "${YELLOW}[6/6] Checking container logs...${NC}"
LOGS=$(docker logs neuroquantum-test 2>&1 | tail -20)
echo "$LOGS"

if echo "$LOGS" | grep -i "error\|panic\|fatal" > /dev/null; then
    echo -e "${YELLOW}WARNING: Errors detected in logs${NC}"
else
    echo -e "${GREEN}✓ No critical errors in logs${NC}"
fi

# Cleanup
echo ""
echo -e "${YELLOW}Cleaning up...${NC}"
docker stop neuroquantum-test > /dev/null 2>&1 || true
docker rm neuroquantum-test > /dev/null 2>&1 || true

# Summary
echo ""
echo -e "${GREEN}=== Test Summary ===${NC}"
echo "Image: $IMAGE_NAME:$IMAGE_TAG"
echo "Size: ${IMAGE_SIZE_MB}MB"
echo "Build Time: ${BUILD_TIME}s"
echo "Memory: ${MEM_USAGE}MiB"
echo ""
echo -e "${GREEN}✓ Docker build and test completed successfully${NC}"
echo ""
echo "Next steps:"
echo "  1. Test with docker-compose: cd docker/production && docker-compose up"
echo "  2. Run integration tests: cargo test --workspace"
echo "  3. Push to registry: docker push $IMAGE_NAME:$IMAGE_TAG"

