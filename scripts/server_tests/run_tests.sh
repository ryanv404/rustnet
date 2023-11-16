#!/usr/bin/bash

trap "clean_up" INT TERM ERR

# Test parameters.
ALL_TESTS=(
    "get_index GET /"
    "get_about GET /about"
    "get_foo GET /foo"
    "head_index HEAD /"
    "head_about HEAD /about"
    "head_foo HEAD /foo"
    "head_favicon HEAD /favicon.ico"
    "post_about POST /about"
    "put_about PUT /about"
    "patch_about PATCH /about"
    "delete_about DELETE /about"
    "trace_about TRACE /about"
    "options_about OPTIONS /about"
    "connect_about CONNECT /about"
)

# Global variables.
SERVER_PID=0
NUM_PASSED=0
NUM_TESTS="${#ALL_TESTS[@]}"
CRATE_DIR="$(pwd)"
TESTS_DIR="${CRATE_DIR}/scripts/server_tests"
TEMP_FILE="${TESTS_DIR}/temp.txt"
SERVER_BIN="${CRATE_DIR}/target/debug/examples/server"
SERVER_ADDR='http://127.0.0.1:7878'

# Terminal colors.
RED=$'\e[91m'
GRN=$'\e[92m'
YLW=$'\e[93m'
BLU=$'\e[94m'
PURP=$'\e[95m'
CYAN=$'\e[96m'
CLR=$'\e[0m'

build_server() {
    cargo clean &> /dev/null

    if (( $? != 0 )); then
        echo -e "${YLW}Unable to remove prior build artifacts.${CLR}"
    fi

    echo -n "Building..."

    cargo build --example server &> /dev/null

    if (( $? != 0 )); then
        echo -e "${RED}✗ Unable to build the server.${CLR}"
        clean_up
    elif [[ ! -x "$SERVER_BIN" ]]; then
        echo -e "${RED}✗ Unable to execute the server binary.${CLR}"
        clean_up
    else
        echo -e "${GRN}✔${CLR}"
    fi
}

start_server() {
    echo -n "Starting..."

	"$SERVER_BIN" &> /dev/null &

    SERVER_PID="$!"

	# Give the server a little time to go live.
	sleep 2

    echo -e "${GRN}✔${CLR}"
}

confirm_server_is_live() {
    local ATTEMPT_NUM=0
    local MAX_ATTEMPTS=5
    local STILL_CONNECTING=0

    while (( ($ATTEMPT_NUM < $MAX_ATTEMPTS) && ($STILL_CONNECTING == 0) )); do
        echo -n "Connecting..."

        curl --silent -X GET "$SERVER_ADDR" &> /dev/null

        if (( $? == 0 )); then
            echo -e "${GRN}✔${CLR}\n"
            STILL_CONNECTING=1
            break
        else
            echo -e "${YLW}Not live.${CLR}"
            (( ATTEMPT_NUM++ ))
            sleep 1
        fi
    done

    if (( $STILL_CONNECTING == 0 )); then
        echo -e "${RED}✗ The server is unreachable.${CLR}"
        clean_up
    fi
}

print_result() {
    local TEST_NAME="$1"
    local TEST_RESULT="$2"
    local TEST_LABEL="$3"
    local EXPECTED_FILE="${TESTS_DIR}/${TEST_NAME}.txt"

    if [[ ! -f "$EXPECTED_FILE" ]]; then
        echo -e -n "[${RED}✗${CLR}] ${BLU}${TEST_LABEL}${CLR}: "
        echo -e "${RED}No expected output file found.${CLR}"
        return
    fi

    if [[ -z "$TEST_RESULT" ]]; then
        echo -e -n "[${RED}✗${CLR}] ${BLU}${TEST_LABEL}${CLR}: "
        echo -e "${RED}No response received.${CLR}"
        return
    fi

    echo -n "$TEST_RESULT" > "$TEMP_FILE"

    # Test if the test output and expected output are identical, ignoring differences
    # that only involve blank lines or trailing whitespace.
    diff --ignore-blank-lines --ignore-trailing-space "$TEMP_FILE" \
        "$EXPECTED_FILE" &> /dev/null

    if (( $? == 0 )); then
        # The test passed.
        echo -e "[${GRN}✔${CLR}] ${BLU}${TEST_LABEL}${CLR}"
        (( NUM_PASSED++ ))
    elif (( $? == 1 )); then
        # If not identical, get a side-by-side diff that ignores various whitespace
        # false positives and only shows the lines with differences.
        local DIFFS=$( diff --color=always --ignore-blank-lines \
            --ignore-trailing-space --side-by-side --suppress-common-lines \
            --width=80 "$TEMP_FILE" "$EXPECTED_FILE" )

        echo -e -n "[${RED}✗${CLR}] ${BLU}${TEST_LABEL}${CLR}: "
        echo -e "${RED}Did not match the expected output.${CLR}"
        echo -e "[${PURP}<--OUTPUT--${CLR} | ${PURP}--EXPECTED-->${CLR}]"
        echo -e "${DIFFS}\n"
    else
        # There was an unexpected error with the diff command.
        echo -e -n "[${RED}✗${CLR}] ${BLU}${TEST_LABEL}${CLR}: "
        echo -e "${RED}Error comparing the test and expected outputs.${CLR}"
    fi
}

clean_up() {
    rm -f "$TEMP_FILE" &> /dev/null

    if [[ ! -z "$SERVER_PID" ]]; then
        ps -p "$SERVER_PID" &> /dev/null

        if (( $? == 0 )); then
            kill -SIGTERM "$SERVER_PID" &> /dev/null
            wait -f "$SERVER_PID" &> /dev/null
        fi
    fi

    cargo clean &> /dev/null

    if (( $? != 0 )); then
         echo -e "${YLW}Unable to remove build artifacts.${CLR}"
    fi

    unset ALL_TESTS NUM_TESTS NUM_PASSED SERVER_PID SERVER_ADDR SERVER_BIN
    unset CRATES_DIR TESTS_DIR TEMP_FILE RED GRN BLU CYAN YLW PURP CLR

    exit
}

# Run a single test.
run_one_test() {
    local TEST_RESULT=""
    local TEST_NAME="$1"
    local TEST_METHOD="$2"
    local TEST_URI="$3"
    local TEST_LABEL="$TEST_METHOD $TEST_URI"
    local ADDR="${SERVER_ADDR}${TEST_URI}"

    case "$TEST_METHOD" in
    "GET")
        TEST_RESULT=$( curl --silent --include -X GET "$ADDR" );;
    "HEAD")
        TEST_RESULT=$( curl --silent --head "$ADDR" );;
    "POST")
        TEST_RESULT=$( curl --silent --include -X POST "$ADDR" );;
    "PUT")
        TEST_RESULT=$( curl --silent --include -X PUT "$ADDR" );;
    "PATCH")
        TEST_RESULT=$( curl --silent --include -X PATCH "$ADDR" );;
    "DELETE")
        TEST_RESULT=$( curl --silent --include -X DELETE "$ADDR" );;
    "TRACE")
        TEST_RESULT=$( curl --silent --include -X TRACE "$ADDR" );;
    "OPTIONS")
        TEST_RESULT=$( curl --silent --include -X OPTIONS "$ADDR" );;
    "CONNECT")
        TEST_RESULT=$( curl --silent --include -X CONNECT "$ADDR" );;
    *)
        echo -e -n "[${RED}✗${CLR}] ${BLU}${TEST_LABEL}${CLR}: "
        echo -e "${RED}Invalid HTTP method.${CLR}"
        return;;
    esac

    print_result "$TEST_NAME" "$TEST_RESULT" "$TEST_LABEL"
}

# Builds and starts server, runs all server tests, and cleans up after itself.
run_all_tests() {
    build_server
    start_server
    confirm_server_is_live

    echo "ALL TESTS:"

    for test in "${ALL_TESTS[@]}"; do
        local TEST_ARR=($test)
        local TEST_NAME="${TEST_ARR[0]}"
        local TEST_METHOD="${TEST_ARR[1]}"
        local TEST_URI="${TEST_ARR[2]}"
        run_one_test "$TEST_NAME" "$TEST_METHOD" "$TEST_URI"
    done

    echo -e "\n${BLU}+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+${CLR}"

    if (( ($NUM_TESTS == $NUM_PASSED) && ($NUM_TESTS > 0) )); then
        echo -e "${GRN}${NUM_PASSED} / ${NUM_TESTS} tests passed.${CLR}"
    else
        echo -e "${RED}${NUM_PASSED} / ${NUM_TESTS} tests passed.${CLR}"
    fi

    echo -e "${BLU}+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+${CLR}"

    clean_up
}


# Run all server tests.
run_all_tests
