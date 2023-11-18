#!/usr/bin/bash

trap "clean_up" INT TERM ERR

# Common global variables.
CRATE_DIR="$(pwd)"
TEMP_FILE="${CRATE_DIR}/scripts/temp.txt"

# Server global variables.
SERVER_PID=0
TOTAL_SERVER_TESTS=0
SERVER_NUM_PASSED=0
SERVER_BIN="${CRATE_DIR}/target/debug/server"
SERVER_TESTS_DIR="${CRATE_DIR}/scripts/server_tests"

# Client global variables.
TOTAL_CLIENT_TESTS=0
CLIENT_NUM_PASSED=0
CLIENT_BIN="${CRATE_DIR}/target/debug/client"
CLIENT_TESTS_DIR="${CRATE_DIR}/scripts/client_tests"

# Terminal colors.
RED=$'\e[91m'
GRN=$'\e[92m'
YLW=$'\e[93m'
BLU=$'\e[94m'
PURP=$'\e[95m'
CYAN=$'\e[96m'
CLR=$'\e[0m'

# Builds the test server.
build_server() {
    cargo clean &> /dev/null

    if (( $? != 0 )); then
        echo -e "${YLW}Unable to remove prior build artifacts.${CLR}"
    fi

    echo -n "Building server..."

    cargo build --bin server &> /dev/null

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

# Builds the client.
build_client() {
    cargo clean &> /dev/null

    if (( $? != 0 )); then
        echo -e "${YLW}Unable to remove build artifacts.${CLR}"
    fi

    echo -e -n "Building client..."

    cargo build --bin client &> /dev/null

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

# Starts the test server.
start_server() {
    echo -n "Starting server..."

	"$SERVER_BIN" &> /dev/null &

    SERVER_PID="$!"

	# Give the server a little time to boot.
	sleep 1

    echo -e "${GRN}✔${CLR}"

    confirm_server_is_live
}

# Confirms server is live and reachable before starting tests.
confirm_server_is_live() {
    local ATTEMPT_NUM=0
    local MAX_ATTEMPTS=5
    local STILL_NOT_LIVE=1

    while (( ($ATTEMPT_NUM < $MAX_ATTEMPTS) && ($STILL_NOT_LIVE == 1) )); do
        echo -n "Connecting..."

        curl --silent -X GET '127.0.0.1:7878' &> /dev/null

        if (( $? == 0 )); then
            echo -e "${GRN}✔${CLR}\n"
            STILL_NOT_LIVE=0
            break
        else
            echo -e "${YLW}Not live.${CLR}"
            (( ATTEMPT_NUM++ ))
            sleep 1
        fi
    done

    if (( $STILL_NOT_LIVE == 1 )); then
        echo -e "\n${RED}✗ The server is unreachable.${CLR}"
        clean_up
    fi
}

# Compares the server test's output to the expected output.
get_server_test_result() {
    local RES="$1"
    local LABEL="$2"
    local FILE="$3"
    local EXP_FILE="${SERVER_TESTS_DIR}/$FILE"

    if [[ ! -f "$EXP_FILE" ]]; then
        echo -e -n "[${RED}✗${CLR}] ${BLU}${LABEL}${CLR}: "
        echo -e "${RED}No expected output file found.${CLR}"
        return
    fi

    if [[ -z "$RES" ]]; then
        echo -e -n "[${RED}✗${CLR}] ${BLU}${LABEL}${CLR}: "
        echo -e "${RED}No response received.${CLR}"
        return
    fi

    echo -n "$RES" > "$TEMP_FILE"

    # Test if the test output and expected output are identical, ignoring differences
    # that only involve blank lines or trailing whitespace.
    diff --text \
        --ignore-blank-lines \
        --ignore-trailing-space \
        "$TEMP_FILE" \
        "$EXP_FILE" \
        &> /dev/null

    if (( $? == 0 )); then
        # The test passed.
        echo -e "[${GRN}✔${CLR}] ${BLU}${LABEL}${CLR}"
        (( SERVER_NUM_PASSED++ ))
    elif (( $? == 1 )); then
        # If not identical, get a side-by-side diff that ignores various whitespace
        # false positives and only shows the lines with differences.
        local DIFFS=$(diff --text \
            --ignore-blank-lines \
            --ignore-trailing-space \
            --color=always \
            --side-by-side \
            --suppress-common-lines \
            --width=80 \
            "$TEMP_FILE" \
            "$EXP_FILE")

        echo -e -n "[${RED}✗${CLR}] ${BLU}${LABEL}${CLR}: "
        echo -e "${RED}Did not match the expected output.${CLR}"

        echo -e "[${PURP}<--OUTPUT--${CLR} | ${PURP}--EXPECTED-->${CLR}]"
        echo -e "${DIFFS}\n"
    else
        # There was an unexpected error with the diff command.
        echo -e -n "[${RED}✗${CLR}] ${BLU}${LABEL}${CLR}: "
        echo -e "${RED}Error comparing the test and expected outputs.${CLR}"
    fi
}

# Compares the client test's output to the expected output.
get_client_test_result() {
    local RES="$1"
    local LABEL="$2"
    local FILE="$3"
    local EXP_FILE="${CLIENT_TESTS_DIR}/$FILE"

    if [[ ! -f "$EXP_FILE" ]]; then
        echo -e -n "[${RED}✗${CLR}] ${BLU}${LABEL}${CLR}: "
        echo -e "${RED}No expected output file found.${CLR}"
        return
    fi

    if [[ -z "$RES" ]]; then
        echo -e -n "[${RED}✗${CLR}] ${BLU}${LABEL}${CLR}: "
        echo -e "${RED}No response received.${CLR}"
        return
    fi

    echo -n "$RES" > "$TEMP_FILE"

    # Compare the output to the expected output, ignoring differences that are
    # only blank lines or trailing whitespace.
    diff --text \
        --ignore-blank-lines \
        --ignore-trailing-space \
        "$TEMP_FILE" \
        "$EXP_FILE" \
        &> /dev/null

    if (( $? == 0 )); then
        # The test passed.
        echo -e "[${GRN}✔${CLR}] ${BLU}${LABEL}${CLR}"
        (( CLIENT_NUM_PASSED++ ))
    elif (( $? == 1 )); then
        # No match so display only the lines that are different.
        local DIFFS=$(diff --text \
            --ignore-blank-lines \
            --ignore-trailing-space \
            --color=always \
            --side-by-side \
            --suppress-common-lines \
            --width=80 \
            "$TEMP_FILE" \
            "$EXP_FILE")

        echo -e -n "[${RED}✗${CLR}] ${BLU}${LABEL}${CLR}: "
        echo -e "${RED}Did not match the expected output.${CLR}"

        echo -e "[${PURP}<--OUTPUT--${CLR} | ${PURP}--EXPECTED-->${CLR}]"
        echo -e "${DIFFS}\n"
    else
        # There was an unexpected error with diff.
        echo -e -n "[${RED}✗${CLR}] ${BLU}${LABEL}${CLR}: "
        echo -e "${RED}Error comparing the test and expected outputs.${CLR}"
    fi
}

# Runs one server test.
run_one_server_test() {
    local RES=""
    local METHOD="$1"
    local URI="$2"
    local LABEL="$3"
    local FILE="$4"
    local URL="127.0.0.1:7878${URI}"

    case "$METHOD" in
    "GET")
        RES=$( curl --silent --include -X GET "$URL" );;
    "HEAD")
        RES=$( curl --silent --head "$URL" );;
    "POST")
        RES=$( curl --silent --include -X POST "$URL" );;
    "PUT")
        RES=$( curl --silent --include -X PUT "$URL" );;
    "PATCH")
        RES=$( curl --silent --include -X PATCH "$URL" );;
    "DELETE")
        RES=$( curl --silent --include -X DELETE "$URL" );;
    "TRACE")
        RES=$( curl --silent --include -X TRACE "$URL" );;
    "OPTIONS")
        RES=$( curl --silent --include -X OPTIONS "$URL" );;
    "CONNECT")
        RES=$( curl --silent --include -H "Host: 127.0.0.1" \
            --request-target "$URI" -X CONNECT "$URI" );;
    *)
        echo -e -n "[${RED}✗${CLR}] ${BLU}${LABEL}${CLR}: "
        echo -e "${RED}Invalid HTTP method.${CLR}"
        return;;
    esac

    get_server_test_result "$RES" "$LABEL" "$FILE"
}

# Runs one client test.
run_one_client_test() {
    local METHOD="$1"
    local URI="$2"
    local LABEL="$3"
    local FILE="$4"

	local RES=$("$CLIENT_BIN" --testing 'httpbin.org' "$URI" 2> /dev/null)

    get_client_test_result "$RES" "$LABEL" "$FILE"
}

# Builds, starts, and tests the server.
run_all_server_tests() {
    build_server
    start_server

    echo "SERVER TESTS:"

    # Parse test parameters from the file names.
    TESTS=($(find "$SERVER_TESTS_DIR" -type f -name "*.txt" -print0 | \
        xargs -0 -I {} basename --suffix ".txt" "{}" | \
        tr '\n' ' '))

    for test in "${TESTS[@]}"; do
        (( TOTAL_SERVER_TESTS++ ))

        local METHOD=""
        local URI=""
        local LABEL=""
        local FILE=""

        METHOD="${test%_*}"
        METHOD="${METHOD^^}"

        URI="${test#*_}"
        URI="${URI,,}"

        if [[ "$URI" == "index" ]]; then
            URI="/"
        elif [[ "$URI" == "favicon" ]]; then
            URI="/favicon.ico"
        elif [[ "$METHOD" != "CONNECT" ]]; then
            URI="/$URI"
        fi

        LABEL="$METHOD $URI"
        FILE="${test}.txt"

        run_one_server_test "$METHOD" "$URI" "$LABEL" "$FILE"
    done

    echo -e "\n${BLU}+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+${CLR}"

    local PASSED = $SERVER_NUM_PASSED
    local TOTAL = $TOTAL_SERVER_TESTS

    if (( ($TOTAL == $PASSED) && ($TOTAL > 0) )); then
        echo -e "${GRN}${PASSED} of ${TOTAL} server tests passed.${CLR}"
    else
        echo -e "${RED}${PASSED} of ${TOTAL} server tests passed.${CLR}"
    fi

    echo -e "${BLU}+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+${CLR}"

    clean_up
}

# Builds and tests the client.
run_all_client_tests() {
    build_client

    echo "CLIENT TESTS:"

    # Parse test parameters from the file names.
    TESTS=($(find "$CLIENT_TESTS_DIR" -type f -name "*.txt" -print0 | \
        xargs -0 -I {} basename --suffix ".txt" "{}" | \
        tr '\n' ' '))

    for test in "${TESTS[@]}"; do
        (( TOTAL_CLIENT_TESTS++ ))

        local METHOD=""
        local URI=""
        local LABEL=""
        local FILE=""

        METHOD="${test%_*}"
        METHOD="${METHOD^^}"

        URI="${test#*_}"
        URI="${URI,,}"

        case "$URI" in
        "jpeg")
            URI="/image/jpeg";;
        "png")
            URI="/image/png";;
        "svg")
            URI="/image/svg";;
        "text")
            URI="/robots.txt";;
        "utf8")
            URI="/encoding/utf8";;
        "webp")
            URI="/image/webp";;
        *)
            URI="/$URI";;
        esac

        LABEL="$METHOD $URI"
        FILE="${test}.txt"

        run_one_client_test "$METHOD" "$URI" "$LABEL" "$FILE"
    done

    echo -e "\n${BLU}+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+${CLR}"

    local PASSED = $CLIENT_NUM_PASSED
    local TOTAL = $TOTAL_CLIENT_TESTS

    if (( ($TOTAL == $PASSED) && ($TOTAL > 0) )); then
        echo -e "${GRN}${PASSED} of ${TOTAL} client tests passed.${CLR}"
    else
        echo -e "${RED}${PASSED} of ${TOTAL} client tests passed.${CLR}"
    fi

    echo -e "${BLU}+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+${CLR}"

    clean_up
}

# Clean up build artifacts and testing debris.
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
}

# Prints a help message to the terminal.
print_help() {
    echo -e "${GRN}USAGE${CLR}"
    echo -e "    $(basename $0) <ARGUMENT>\n"
    echo -e "${GRN}ARGUMENTS${CLR}"
    echo -e "    all      Run all tests."
    echo -e "    client   Run all client tests only."
    echo -e "    server   Run all server tests only.\n"
}

# Handle command line arguments.
if (( $# < 1 )); then
    echo -e "${YLW}Please select a test group to run.${CLR}\n"
    print_help
else
    CLI_ARG="${1,,}"

    case "$CLI_ARG" in
    "all")
        run_all_client_tests
        echo
        run_all_server_tests;;
    "client")
        run_all_client_tests;;
    "server")
        run_all_server_tests;;
    *)
        echo -e "${YLW}Unknown argument: \"${1}\"${CLR}\n"
        print_help;;
    esac
fi

unset TOTAL_SERVER_TESTS SERVER_NUM_PASSED SERVER_PID SERVER_BIN CLIENT_BIN
unset CRATE_DIR TESTS_DIR TEMP_FILE RED GRN BLU CYAN YLW PURP CLR
unset SERVER_TESTS_DIR CLIENT_TESTS_DIR

exit
