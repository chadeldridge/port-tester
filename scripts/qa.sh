#!/usr/bin/env bash
# Full QA Functional Test for port-tester

set -e

parent=$(basename "$(pwd)")
if [[ "$parent" == "scripts" ]]; then
    cd ../
fi

PASS=0
FAIL=0
TOTAL=0

update_pass () {
    PASS=$((PASS+1))
    TOTAL=$((TOTAL+1))
}

update_fail () {
    FAIL=$((FAIL+1))
    TOTAL=$((TOTAL+1))
}

print_results () {
    echo "${1} test result: ${PASS} passed; ${FAIL} failed; ${TOTAL} total"
    echo
}

echo "Building release binaries for QA..."
cargo build --release
set +e

PT_BIN="./target/release/pt"
POKE_BIN="./target/release/poke"

echo
echo "pt tests"
echo -n "pt - test 1: basic success (8.8.8.8:53) ... "
if $PT_BIN 8.8.8.8 53 -c 1 > /dev/null; then
    update_pass
    echo -e "\033[32mok\033[0m"
else
    update_fail
    echo -e "\033[31mfail\033[0m"
fi

echo -n "pt - test 2: basic failure (127.0.0.1:1) ... "
# Using 127.0.0.1:1 which is almost certainly closed.
if ! $PT_BIN 127.0.0.1 1 -c 1 -t 1 > /dev/null; then
    update_pass
    echo -e "\033[32mok\033[0m"
else
    update_fail
    echo -e "\033[31mfail\033[0m: failure case incorrectly exited with 0"
fi

echo -n "pt - test 3: silent mode performance ... "
OUTPUT=$($PT_BIN -s 8.8.8.8 53 -c 1)
if [ -z "$OUTPUT" ]; then
    update_pass
    echo -e "\033[32mok\033[0m"
else
    update_fail
    echo -e "\033[31mfail\033[0m: silent mode produced output: $OUTPUT"
fi

echo -n "pt - test 4: count verification ... "
# Check if we get the right number of lines for a count of 3
LINES=$($PT_BIN 8.8.8.8 53 -c 3 | grep -c "ok" || true)
if [ "$LINES" -ge 3 ]; then
    update_pass
    echo -e "\033[32mok\033[0m"
else
    update_fail
    echo -e "\033[31mfail\033[0m: expected at least 3 'ok' lines, got $LINES"
fi

echo -n "pt - test 5: json output format ... "
JSON_OUTPUT=$($PT_BIN 8.8.8.8 53 -c 1 --json)
if echo "$JSON_OUTPUT" | grep -q "\"addrs\"" && echo "$JSON_OUTPUT" | grep -q "\"metrics\""; then
    update_pass
    echo -e "\033[32mok\033[0m"
else
    update_fail
    echo -e "\033[31mfail\033[0m: json output missing 'addrs' or 'metrics' keys"
    echo "output was: $JSON_OUTPUT"
fi

echo -n "pt - test 6: interval timing (dry run) ... "
START=$(date +%s)
$PT_BIN 8.8.8.8 53 -c 2 -i 2 > /dev/null
END=$(date +%s)
DIFF=$((END-START))
if [ "$DIFF" -ge 2 ]; then
    update_pass
    echo -e "\033[32mok\033[0m"
else
    update_fail
    echo -e "\033[31mfail\033[0m: timing was too fast (${DIFF}s), interval might be ignored"
fi

# Print results for pt tests
print_results "pt"

# Reset counters for next set of tests.
PASS=0
FAIL=0
TOTAL=0

echo "poke tests"
echo -n "poke - test 1: basic success (8.8.8.8:53) ... "
if $POKE_BIN 8.8.8.8 53 > /dev/null; then
    update_pass
    echo -e "\033[32mok\033[0m"
else
    update_fail
    echo -e "\033[31mfail\033[0m"
fi

echo -n "poke - test 2: basic failure (127.0.0.1:1) ... "
# Using 127.0.0.1:1 which is almost certainly closed.
if ! $POKE_BIN 127.0.0.1 1 -t 1 > /dev/null; then
    update_pass
    echo -e "\033[32mok\033[0m"
else
    update_fail
    echo -e "\033[31mfail\033[0m: failure case incorrectly exited with 0"
fi

echo -n "poke - test 3: quiet mode performance ... "
# Using 127.0.0.1:1 which is almost certainly closed.
OUTPUT=$($POKE_BIN -q 127.0.0.1 1 -t 1)
if [[ "$OUTPUT" == "fail" ]]; then
    update_pass
    echo -e "\033[32mok\033[0m"
else
    update_fail
    echo -e "\033[31mfail\033[0m: quiet mode produced output: '$OUTPUT'"
fi

echo -n "poke - test 4: silent mode performance ... "
OUTPUT=$($POKE_BIN -s 8.8.8.8 53)
if [ -z "$OUTPUT" ]; then
    update_pass
    echo -e "\033[32mok\033[0m"
else
    update_fail
    echo -e "\033[31mfail\033[0m: silent mode produced output: $OUTPUT"
fi

echo -n "poke - test 5: count verification ... "
# Check if we get the right number of lines for a count of 3
LINES=$($POKE_BIN 8.8.8.8 53 | grep -c "ok" || true)
if [ "$LINES" -ge 1 ]; then
    update_pass
    echo -e "\033[32mok\033[0m"
else
    update_fail
    echo -e "\033[31mfail\033[0m: expected 1 'ok' line, got $LINES"
fi

# Print results for pt tests
print_results "poke"
