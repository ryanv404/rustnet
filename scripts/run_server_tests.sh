#!/usr/bin/bash

trap "clean_up" INT TERM ERR

# SERVER_ADDR="localhost:7878"

CRATE_DIR="/data/data/com.termux/files/home/projects/rustnet"
SERVER_BIN="${CRATE_DIR}/target/debug/examples/server"
TEST_OUTPUT_FILE="${CRATE_DIR}/scripts/test_output.txt"
EXPECTED_OUTPUT_FILE="${CRATE_DIR}/scripts/expected_output.txt"

SERVER_PID=""

run_all_tests() {
    launch_test_server

    run_test "Get index page test" "/"
    run_test "Get about page test" "/about"
    run_test "Get non-existent page test" "/foo"
    run_test "Get favicon icon test" "/favicon.ico"

    analyze_test_results

    clean_up
}

build_test_server() {
    echo "Building the test server."

    cargo build --binary server &> /dev/null

    if [[ "$?" -ne 0 ]]; then
        echo "Unable to build the test server."
        clean_up
    fi
}

launch_test_server() {
    if [[ -x "$SERVER_BIN" ]]; then
        echo "Cleaning prior build artifacts."
        cargo clean &> /dev/null
    fi

    build_test_server

    echo "Launching the test server."

    if [[ -f "$TEST_OUTPUT_FILE" ]]; then
        rm "$TEST_OUTPUT_FILE"
    fi

    touch "$TEST_OUTPUT_FILE"

	cargo run --binary server &> /dev/null &

	# Give server time to go live.
	sleep 3

    SERVER_PID="$!"

    echo "The test server is running with PID ${SERVER_PID}."
}

parse_and_print_result() {
    local TEST_NAME="$1"
    local TEST_RESULT="$2"

    if [[ ! -z "$TEST_RESULT" ]]; then
        echo -e "\n[${TEST_NAME}]:"
        echo "$TEST_RESULT"
        echo "$TEST_RESULT" >> "$TEST_OUTPUT_FILE"
    else
        echo "No output received for this test."
    fi
}

clean_up() {
    if [[ ! -z "$SERVER_PID" ]]; then
        ps aux | rg --quiet "$SERVER_PID"

        if [[ "$?" -eq 0 ]]; then 
            kill -SIGTERM "$SERVER_PID"
            wait -f "$SERVER_PID"
        fi

        echo -e "\nThe test server with PID $SERVER_PID has been closed."
    fi

    if [[ -x "$SERVER_BIN" ]]; then
        echo "Finishing clean up and then exiting."
        cargo clean &> /dev/null
    fi

    exit
}

analyze_test_results() {
    local GREEN=$'\e[32m'
    local RED=$'\e[31m'
    local RESET=$'\e[0m'

    diff --report-identical-files --text "$TEST_OUTPUT_FILE" "$EXPECTED_OUTPUT_FILE" > /dev/null

    if [[ "$?" -eq 0 ]]; then
        echo -e "\n${GREEN}✔ ALL TESTS PASSED!${RESET}"
    else
        echo -e "\n${RED}✗ THERE WERE TEST FAILURES :-(${RESET}"
    fi
}

run_test() {
    local TEST_NAME="$1"
    local TEST_TARGET="$2"
    local TEST_RESULT=$( xh --print=h --no-check-status "${SERVER_ADDR}${TEST_TARGET}" )

    parse_and_print_result "$TEST_NAME" "$TEST_RESULT"
}

run_all_tests
