#!/bin/bash
# End-to-end test for magic link authentication flow
#
# Prerequisites:
# 1. Docker (for Mailhog)
# 2. Built todufit binaries
#
# Usage:
#   ./hack/test-auth-flow.sh

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "=== ToduFit Magic Link Auth E2E Test ==="
echo

# Create temp directory for test data
TEST_DIR=$(mktemp -d)
echo "Test directory: $TEST_DIR"

# Config paths
SERVER_DATA_DIR="$TEST_DIR/server-data"
SERVER_CONFIG="$TEST_DIR/server-config.yaml"
CLIENT_CONFIG="$TEST_DIR/client-config.yaml"

mkdir -p "$SERVER_DATA_DIR"

# Cleanup on exit
cleanup() {
    echo
    echo "Cleaning up..."
    # Stop Mailhog if we started it
    if [ -n "$MAILHOG_PID" ]; then
        docker stop mailhog-test 2>/dev/null || true
    fi
    # Stop server if running
    if [ -n "$SERVER_PID" ]; then
        kill $SERVER_PID 2>/dev/null || true
    fi
    rm -rf "$TEST_DIR"
    echo "Done."
}
trap cleanup EXIT

# Step 1: Start Mailhog
echo -e "${YELLOW}Step 1: Starting Mailhog...${NC}"
docker run -d --name mailhog-test -p 1025:1025 -p 8025:8025 mailhog/mailhog >/dev/null 2>&1
MAILHOG_PID=1
sleep 2
echo -e "${GREEN}✓ Mailhog running at http://localhost:8025${NC}"
echo

# Step 2: Create server config
echo -e "${YELLOW}Step 2: Creating server config...${NC}"
cat > "$SERVER_CONFIG" << EOF
auth:
  smtp_host: localhost
  smtp_port: 1025
  from_email: noreply@todufit.local
  from_name: ToduFit
  server_url: http://localhost:8080
  token_expiry_minutes: 10
EOF
echo -e "${GREEN}✓ Server config created${NC}"
echo

# Step 3: Add test user
echo -e "${YELLOW}Step 3: Adding test user...${NC}"
TODUFIT_DATA_DIR="$SERVER_DATA_DIR" cargo run --bin todufit-admin -- \
    user add test@example.com --group testgroup --name "Test User"
echo -e "${GREEN}✓ Test user added${NC}"
echo

# Step 4: Start server
echo -e "${YELLOW}Step 4: Starting server...${NC}"
TODUFIT_DATA_DIR="$SERVER_DATA_DIR" \
TODUFIT_CONFIG="$SERVER_CONFIG" \
TODUFIT_PORT=8080 \
cargo run --bin todufit-server &
SERVER_PID=$!
sleep 3

# Check if server is running
if curl -s http://localhost:8080/health | grep -q "ok"; then
    echo -e "${GREEN}✓ Server running at http://localhost:8080${NC}"
else
    echo -e "${RED}✗ Server failed to start${NC}"
    exit 1
fi
echo

# Step 5: Create client config
echo -e "${YELLOW}Step 5: Creating client config...${NC}"
cat > "$CLIENT_CONFIG" << EOF
sync:
  server_url: ws://localhost:8080
EOF
echo -e "${GREEN}✓ Client config created${NC}"
echo

# Step 6: Test login request
echo -e "${YELLOW}Step 6: Testing login request...${NC}"
RESPONSE=$(curl -s -X POST http://localhost:8080/auth/login \
    -H "Content-Type: application/json" \
    -d '{"email":"test@example.com","callback_url":"http://localhost:9999/callback"}')

if echo "$RESPONSE" | grep -q '"status":"ok"'; then
    echo -e "${GREEN}✓ Login request accepted${NC}"
else
    echo -e "${RED}✗ Login request failed: $RESPONSE${NC}"
    exit 1
fi
echo

# Step 7: Check email was sent
echo -e "${YELLOW}Step 7: Checking Mailhog for email...${NC}"
sleep 1
EMAILS=$(curl -s http://localhost:8025/api/v2/messages)
if echo "$EMAILS" | grep -q "Sign in to ToduFit"; then
    echo -e "${GREEN}✓ Magic link email sent${NC}"
    
    # Extract the token from the email
    TOKEN=$(echo "$EMAILS" | grep -o 'token=[^"]*' | head -1 | cut -d= -f2)
    echo "  Token: ${TOKEN:0:20}..."
else
    echo -e "${RED}✗ No email found${NC}"
    exit 1
fi
echo

# Step 8: Test token verification
echo -e "${YELLOW}Step 8: Testing token verification...${NC}"
VERIFY_RESPONSE=$(curl -s -w "\n%{http_code}" "http://localhost:8080/auth/verify?token=$TOKEN")
HTTP_CODE=$(echo "$VERIFY_RESPONSE" | tail -1)

if [ "$HTTP_CODE" = "307" ] || [ "$HTTP_CODE" = "303" ]; then
    echo -e "${GREEN}✓ Token verified, redirect received${NC}"
else
    echo "Response code: $HTTP_CODE"
    echo "$VERIFY_RESPONSE"
fi
echo

# Step 9: Test unknown email
echo -e "${YELLOW}Step 9: Testing unknown email...${NC}"
UNKNOWN_RESPONSE=$(curl -s -X POST http://localhost:8080/auth/login \
    -H "Content-Type: application/json" \
    -d '{"email":"unknown@example.com","callback_url":"http://localhost:9999/callback"}')

if echo "$UNKNOWN_RESPONSE" | grep -q '"error":"unknown_user"'; then
    echo -e "${GREEN}✓ Unknown email correctly rejected${NC}"
else
    echo -e "${RED}✗ Expected unknown_user error, got: $UNKNOWN_RESPONSE${NC}"
    exit 1
fi
echo

# Step 10: Test token reuse
echo -e "${YELLOW}Step 10: Testing token reuse (should fail)...${NC}"
REUSE_RESPONSE=$(curl -s "http://localhost:8080/auth/verify?token=$TOKEN")
if echo "$REUSE_RESPONSE" | grep -q "Invalid or expired"; then
    echo -e "${GREEN}✓ Token correctly rejected on reuse${NC}"
else
    echo -e "${RED}✗ Token should have been rejected${NC}"
    exit 1
fi
echo

# Step 11: Test auth status CLI
echo -e "${YELLOW}Step 11: Testing auth status CLI...${NC}"
STATUS=$(cargo run --bin todufit -- -c "$CLIENT_CONFIG" auth status 2>&1)
if echo "$STATUS" | grep -q "Not logged in"; then
    echo -e "${GREEN}✓ Auth status shows not logged in${NC}"
else
    echo "Status: $STATUS"
fi
echo

echo "=== All tests passed! ==="
echo
echo "Manual testing notes:"
echo "1. Open http://localhost:8025 to view sent emails"
echo "2. Run: cargo run --bin todufit -- -c $CLIENT_CONFIG auth login"
echo "3. Enter: test@example.com"
echo "4. Check Mailhog and click the magic link"
echo "5. Verify authentication completes"
