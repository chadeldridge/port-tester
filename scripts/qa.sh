#!/usr/bin/env bash
# Full QA Functional Test for port-tester

set -e

parent=$(basename "$(pwd)")
if [[ "$parent" == "scripts" ]]; then
    cd ../
fi

echo "Building release binary for QA..."
cargo build --release
PT_BIN="./target/release/pt"

echo "------------------------------------------------"
echo "Test 1: Basic Success (8.8.8.8:53)"
if $PT_BIN 8.8.8.8 53 -c 1 > /dev/null; then
    echo "PASS: Success case exited with 0"
else
    echo "FAIL: Success case exited with $?"
    exit 1
fi

echo "------------------------------------------------"
echo "Test 2: Basic Failure (Local Port 1)"
# Using 127.0.0.1:1 which is almost certainly closed.
if ! $PT_BIN 127.0.0.1 1 -c 1 > /dev/null; then
    echo "PASS: Failure case exited with non-zero"
else
    echo "FAIL: Failure case incorrectly exited with 0"
    exit 1
fi

echo "------------------------------------------------"
echo "Test 3: Silent Mode Performance"
OUTPUT=$($PT_BIN -s 8.8.8.8 53 -c 1)
if [ -z "$OUTPUT" ]; then
    echo "PASS: Silent mode produced no stdout"
else
    echo "FAIL: Silent mode produced output: $OUTPUT"
    exit 1
fi

echo "------------------------------------------------"
echo "Test 4: Count Verification"
# Check if we get the right number of lines for a count of 3
LINES=$($PT_BIN 8.8.8.8 53 -c 3 | grep -c "ok" || true)
if [ "$LINES" -ge 3 ]; then
    echo "PASS: Correct number of attempts reported"
else
    echo "FAIL: Expected at least 3 'ok' lines, got $LINES"
    exit 1
fi

echo "------------------------------------------------"
echo "Test 5: JSON Output Format"
JSON_OUTPUT=$($PT_BIN 8.8.8.8 53 -c 1 --json)
if echo "$JSON_OUTPUT" | grep -q "\"addrs\"" && echo "$JSON_OUTPUT" | grep -q "\"metrics\""; then
    echo "PASS: JSON output contains expected fields"
else
    echo "FAIL: JSON output missing 'addrs' or 'metrics' keys"
    echo "Output was: $JSON_OUTPUT"
    exit 1
fi

echo "------------------------------------------------"
echo "Test 6: Interval Timing (Dry Run)"
START=$(date +%s)
$PT_BIN 8.8.8.8 53 -c 2 -i 2 > /dev/null
END=$(date +%s)
DIFF=$((END-START))
if [ "$DIFF" -ge 2 ]; then
    echo "PASS: Interval of 2 seconds observed (Total time: ${DIFF}s)"
else
    echo "FAIL: Timing was too fast (${DIFF}s), interval might be ignored"
    exit 1
fi

echo
echo "------------------------------------------------"
echo "QA COMPLETE: All functional tests passed."