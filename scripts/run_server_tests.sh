#!/usr/bin/bash

trap "clean_up" INT TERM ERR

# Global variables.
NUM_TESTS=0
NUM_PASSED=0
SERVER_PID=""
CRATE_DIR="$(pwd)"
SERVER_ADDR="127.0.0.1:7878"

# Terminal colors.
RED=$'\e[31m'
GRN=$'\e[32m'
YLW=$'\e[33m'
BLU=$'\e[34m'
PURP=$'\e[35m'
CYAN=$'\e[36m'
CLR=$'\e[0m'

run_all_tests() {
    build_server

    launch_server
    confirm_server_is_live

    run_test "get_index_linux" "/"
    run_test "get_about_linux" "/about"
    run_test "get_foo_linux" "/foo"
    run_test "get_favicon_headers" "/favicon.ico"

    if (( ($NUM_TESTS == $NUM_PASSED) && ($NUM_TESTS > 0) )); then
        echo -e "\n${GRN}${NUM_PASSED} / ${NUM_TESTS} tests passed.${CLR}"
    else
        echo -e "\n${RED}${NUM_PASSED} / ${NUM_TESTS} tests passed.${CLR}"
    fi

    clean_up
}

build_server() {
    cargo clean &> /dev/null

    if (( $? != 0 )); then
        echo -e "${YLW}Unable to remove prior build artifacts.${CLR}"
    fi

    echo "Building server..."
    cargo build --bin server &> /dev/null

    if (( $? != 0 )); then
        echo -e "\n${RED}Unable to build the server.${CLR}"
        clean_up
    fi
}

launch_server() {
    echo "Starting server..."

	cargo run --bin server &> /dev/null &
    SERVER_PID="$!"

	# Give server a little time to go live.
	sleep 2
}

confirm_server_is_live() {
    local ATTEMPT_NUM=0
    local MAX_ATTEMPTS=5
    local STILL_CONNECTING=0

    while (( ($ATTEMPT_NUM < $MAX_ATTEMPTS) && ($STILL_CONNECTING == 0) )); do
        echo -n "Connecting... "

        xh "$SERVER_ADDR" &> /dev/null

        if (( $? == 0 )); then
            echo -e "${GRN}Server is live!${CLR}\n"
            STILL_CONNECTING=1
            break
        else
            echo -e "${YLW}Not live yet.${CLR}"
            (( ATTEMPT_NUM++ ))
            sleep 1
        fi
    done

    if (( $STILL_CONNECTING == 0 )); then
        echo -e "\n${RED}The server is unreachable.${CLR}"
    fi
}

print_result() {
    local TEST_NAME="$1"
    local TEST_RESULT="$2"
    local EXPECTED_FILE="${CRATE_DIR}/scripts/tests/${TEST_NAME}.txt"

    echo -e -n "[${BLU}${TEST_NAME}${CLR}]: "

    TRIM_RESULT=$( echo -n "$TEST_RESULT" | sed 's/^ *//g' | sed 's/ *$//g' )

    if [[ -z "$TRIM_RESULT" ]]; then
        echo -e "${RED}✗ (No response received for this test).${CLR}"
        return
    fi

    if [[ ! -f "$EXPECTED_FILE" ]]; then
        echo -e "${RED}✗ (No expected output file found for this test).${CLR}"
        return
    fi

    local EXPECTED=$( cat "$EXPECTED_FILE" | sed 's/^ *//g' | sed 's/ *$//g' )

    if [[ -z "$EXPECTED" ]]; then
        echo -e "${RED}✗ (The expected output file is empty for this test).${CLR}"
        return
    fi

    if [[ "$TRIM_RESULT" == "$EXPECTED" ]]; then
        echo -e "${GRN}✔${CLR}"
        (( NUM_PASSED++ ))
    else
        echo -e "${RED}✗ (The test output did not match the expected output).${CLR}"
        echo -e "${YLW}--[EXPECTED]--\n${EXPECTED}${CLR}\n"
        echo -e "${PURP}--[TEST OUTPUT]--\n${TRIM_RESULT}${CLR}"
    fi
}

clean_up() {
    if [[ ! -z "$SERVER_PID" ]]; then
        ps -p "$SERVER_PID" &> /dev/null

        if (( $? == 0 )); then
            kill -SIGTERM "$SERVER_PID" &> /dev/null
            wait -f "$SERVER_PID" &> /dev/null
        fi
    fi

    cargo clean &> /dev/null

    if (( $? != 0 )); then
         echo -e "${YLW}Unable to remove artifacts from this test run.${CLR}"
    fi

    exit
}

# Run a single test.
run_test() {
    local TEST_RESULT=""
    local TEST_NAME="$1"
    local TEST_TARGET="$2"

    (( NUM_TESTS++ ))

    if [[ "$TEST_TARGET" == "/favicon.ico" ]]; then
        TEST_RESULT=$( xh --print=h --no-check-status "${SERVER_ADDR}${TEST_TARGET}" )
    else
        TEST_RESULT=$( xh --print=hb --no-check-status "${SERVER_ADDR}${TEST_TARGET}" )
    fi

    print_result "$TEST_NAME" "$TEST_RESULT"
}

# Run all server tests.
run_all_tests
