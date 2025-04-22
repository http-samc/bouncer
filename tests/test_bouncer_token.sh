#!/bin/bash

# Simple test script for bouncer-token functionality

echo "Testing Bouncer Token functionality"
echo "==============================="

# Start Bouncer with test config in the background
echo "Starting Bouncer without BOUNCER_TOKEN (should use 'secret')..."
cargo run -- --config examples/default/bouncer.config.yaml &
BOUNCER_PID=$!

# Give it a moment to start
sleep 2

echo "Sending request with bouncer-test: header (should be stripped)..."
RESPONSE=$(curl -s -H "bouncer-test: testvalue" -H "normal-header: value" http://localhost:8000)
echo "Response received."

# Kill the first Bouncer instance
kill $BOUNCER_PID
sleep 1

# Start Bouncer with custom token
echo "Starting Bouncer with custom BOUNCER_TOKEN..."
BOUNCER_TOKEN="custom-secret-value" cargo run -- --config examples/default/bouncer.config.yaml &
BOUNCER_PID=$!

# Give it a moment to start
sleep 2

echo "Sending request with bouncer-test: header (should be stripped)..."
RESPONSE=$(curl -s -H "bouncer-test: testvalue" -H "normal-header: value" http://localhost:8000)
echo "Response received."

# Kill the second Bouncer instance
kill $BOUNCER_PID

echo "Test completed."
echo "Note: This test script requires a properly configured mock server."
echo "Use tools like Wireshark or a mock server that logs headers to verify:"
echo "1. Headers starting with 'bouncer' are stripped"
echo "2. The 'bouncer-token' header is added with the correct value" 