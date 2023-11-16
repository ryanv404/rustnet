#!/usr/bin/bash

trap "clean_up" INT TERM ERR

# Test parameters.
ALL_TESTS=(
    "get_deny GET /deny"
    "get_html GET /html"
    "get_jpeg GET /image/jpeg"
    "get_png GET /image/png"
    "get_json GET /json"
    "get_svg GET /image/svg"
    "get_text GET /robots.txt"
    "get_utf8 GET /encoding/utf8"
    "get_webp GET /image/webp"
    "get_xml GET /xml"
)

# Global variables.
NUM_PASSED=0
NUM_TESTS="${#ALL_TESTS[@]}"
CRATE_DIR="$(pwd)"
TESTS_DIR="${CRATE_DIR}/scripts/client_tests"
TEMP_FILE="${TESTS_DIR}/temp.txt"
CLIENT_BIN="${CRATE_DIR}/target/debug/examples/client"

# Terminal colors.
RED=$'\e[91m'
GRN=$'\e[92m'
YLW=$'\e[93m'
BLU=$'\e[94m'
PURP=$'\e[95m'
CYAN=$'\e[96m'
CLR=$'\e[0m'

# Builds the client.
build_client() {
    cargo clean &> /dev/null

    if (( $? != 0 )); then
        echo -e "${YLW}Unable to remove build artifacts.${CLR}"
    fi

    echo -e -n "Building..."

    cargo build --example client &> /dev/null

    if (( $? != 0 )); then
        echo -e "${RED}✗ Unable to build the client.${CLR}"
        clean_up
    elif [[ ! -x "$CLIENT_BIN" ]]; then
        echo -e "${RED}✗ Unable to execute the client binary.${CLR}"
        clean_up
    else
        echo -e "${GRN}✔${CLR}\n"
    fi
}

# Runs a single test.
run_one_test() {
    local TEST_NAME="$1"
    local TEST_METHOD="$2"
    local TEST_URI="$3"
    local TEST_LABEL="$TEST_METHOD $TEST_URI"

	local TEST_RESULT=$("$CLIENT_BIN" --testing "httpbin.org" "$TEST_URI" 2> /dev/null)

    print_result "$TEST_NAME" "$TEST_RESULT" "$TEST_LABEL"
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

    # Compare the output to the expected output, ignoring differences that are
    # only blank lines or trailing whitespace.
    diff --ignore-blank-lines --ignore-trailing-space "$TEMP_FILE" \
        "$EXPECTED_FILE" &> /dev/null

    if (( $? == 0 )); then
        # The test passed.
        echo -e "[${GRN}✔${CLR}] ${BLU}${TEST_LABEL}${CLR}"
        (( NUM_PASSED++ ))
    elif (( $? == 1 )); then
        # No match so display only the lines that are different.
        local DIFFS=$( diff --color=always --ignore-blank-lines \
            --ignore-trailing-space --side-by-side --suppress-common-lines \
            --width=80 "$TEMP_FILE" "$EXPECTED_FILE" )

        echo -e -n "[${RED}✗${CLR}] ${BLU}${TEST_LABEL}${CLR}: "
        echo -e "${RED}Did not match the expected output.${CLR}"
        echo -e "[${PURP}<--OUTPUT--${CLR} | ${PURP}--EXPECTED-->${CLR}]"
        echo -e "${DIFFS}\n"
    else
        # There was an unexpected error with diff.
        echo -e -n "[${RED}✗${CLR}] ${BLU}${TEST_LABEL}${CLR}: "
        echo -e "${RED}Error comparing the test and expected outputs.${CLR}"
    fi
}

# Cleans up build artifacts in the crate.
clean_up() {
    rm -f "$TEMP_FILE" &> /dev/null

    cargo clean &> /dev/null

    if (( $? != 0 )); then
         echo -e "${YLW}Unable to remove build artifacts.${CLR}"
    fi

    unset ALL_TESTS NUM_TESTS NUM_PASSED CRATES_DIR TESTS_DIR TEMP_FILE CLIENT_BIN
    unset RED GRN BLU CYAN YLW PURP CLR
    exit
}

# Builds client, runs all client tests, and cleans up after itself.
run_all_tests() {
    build_client

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

# Run all client tests.
run_all_tests
