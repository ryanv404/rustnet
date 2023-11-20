#!/usr/bin/bash

trap "clean_up" INT TERM ERR

# Global variables.
SERVER_PID=0
CRATE_DIR="$(pwd)"
CLIENT_NUM_PASSED=0
SERVER_NUM_PASSED=0
TOTAL_CLIENT_TESTS=0
TOTAL_SERVER_TESTS=0
CLIENT_BIN="${CRATE_DIR}/target/debug/client"
SERVER_BIN="${CRATE_DIR}/target/debug/server"
CLIENT_TESTS_DIR="${CRATE_DIR}/scripts/client_tests"
SERVER_TESTS_DIR="${CRATE_DIR}/scripts/server_tests"

# Terminal colors.
CLR=$'\e[0m'
RED=$'\e[91m'
GRN=$'\e[92m'
YLW=$'\e[93m'
BLU=$'\e[94m'
PURP=$'\e[95m'
CYAN=$'\e[96m'

# Builds the test server.
build_server() {
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

    echo -e "${GRN}✔${CLR}"
    confirm_server_is_live
}

# Confirms server is live and reachable before starting tests.
confirm_server_is_live() {
    local ATTEMPT_NUM=0
    local MAX_ATTEMPTS=5
    local STILL_NOT_LIVE=1

	sleep 1

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

# Compares the test's output to the expected output.
get_test_result() {
    local OUTPUT="$1"
    local LABEL="$2"
    local EXP_FILE="$3"
    local KIND="$4"

    if [[ -z "$OUTPUT" ]]; then
        echo -e -n "[${RED}✗${CLR}] ${BLU}${LABEL}${CLR}: "
        echo -e "${RED}No response received.${CLR}"
        return
    fi

    if [[ ! -f "$EXP_FILE" ]]; then
        echo -e -n "[${RED}✗${CLR}] ${BLU}${LABEL}${CLR}: "
        echo -e "${RED}No expected output file found.${CLR}"
        return
    fi

    diff --text --ignore-blank-lines --ignore-trailing-space \
        <( echo "$OUTPUT" ) <( cat "$EXP_FILE" ) &> /dev/null

    if (( $? == 0 )); then
        # The test passed.
        echo -e "[${GRN}✔${CLR}] ${BLU}${LABEL}${CLR}"

        if [[ "$KIND" == "CLIENT" ]]; then
            (( CLIENT_NUM_PASSED++ ))
        else
            (( SERVER_NUM_PASSED++ ))
        fi
    elif (( $? == 1 )); then
        # The test failed.
        echo -e -n "[${RED}✗${CLR}] ${BLU}${LABEL}${CLR}: "
        echo -e "${RED}Did not match the expected output.${CLR}"

        # No match so display only the lines that are different.
        echo -e -n "[${YLW}DIFFERENCES${CLR}] ${CYAN}OUTPUT${CLR} VS "
        echo -e "${PURP}EXPECTED${CLR}"

        diff --text --ignore-blank-lines --ignore-trailing-space \
            --suppress-common-lines --side-by-side --color='always' \
            <( echo "$OUTPUT" ) <( cat "$EXP_FILE" )
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
    local TARGET="$2"
    local LABEL="$3"
    local EXP_FILE="$4"
    local ADDR="127.0.0.1:7878${TARGET}"

    case "$METHOD" in
    "GET")
        RES=$( curl --silent --include -X GET "$ADDR" );;
    "HEAD")
        RES=$( curl --silent --head "$ADDR" );;
    "POST")
        RES=$( curl --silent --include -X POST "$ADDR" );;
    "PUT")
        RES=$( curl --silent --include -X PUT "$ADDR" );;
    "PATCH")
        RES=$( curl --silent --include -X PATCH "$ADDR" );;
    "DELETE")
        RES=$( curl --silent --include -X DELETE "$ADDR" );;
    "TRACE")
        RES=$( curl --silent --include -X TRACE "$ADDR" );;
    "OPTIONS")
        RES=$( curl --silent --include -X OPTIONS "$ADDR" );;
    "CONNECT")
        RES=$( curl --silent --include -H 'Host: 127.0.0.1' \
            --request-target '127.0.0.1:7878' -X CONNECT "$ADDR" );;
    *)
        echo -e -n "[${RED}✗${CLR}] ${BLU}${LABEL}${CLR}: "
        echo -e "${RED}Invalid HTTP method.${CLR}"
        return;;
    esac

    get_test_result "$RES" "$LABEL" "$EXP_FILE" "SERVER"
}

# Runs one client test.
run_one_client_test() {
    local RES=""
    local METHOD="$1"
    local TARGET="$2"
    local LABEL="$3"
    local EXP_FILE="$4"

    local ADDR='54.86.118.241:80'
	RES="$( "$CLIENT_BIN" --testing "$ADDR" "$TARGET" 2> /dev/null )"

    get_test_result "$RES" "$LABEL" "$EXP_FILE" "CLIENT"
}

# Builds, starts, and tests the server.
run_server_tests() {
    local TESTS=""

    cargo clean &> /dev/null

    build_server
    start_server

    # Parse test parameters from the file names.
    TESTS=($( find "$SERVER_TESTS_DIR" -type f -name "*.txt" -print0 | \
              xargs -0 -I {} basename --suffix ".txt" "{}" | \
              tr '\n' ' ' ))

    echo "SERVER TESTS:"

    for test in "${TESTS[@]}"; do
        (( TOTAL_SERVER_TESTS++ ))

        local METHOD=""
        local TARGET=""
        local LABEL=""

        local FILE="${test}.txt"
        local EXP_FILE="${SERVER_TESTS_DIR}/${FILE}"

        METHOD="${test%_*}"
        METHOD="${METHOD^^}"

        TARGET="${test#*_}"
        TARGET="${TARGET,,}"

        if [[ "$TARGET" == "index" ]]; then
            TARGET="/"
        elif [[ "$TARGET" == "favicon" ]]; then
            TARGET="/favicon.ico"
        else
            TARGET="/$TARGET"
        fi

        LABEL="$METHOD $TARGET"
        run_one_server_test "$METHOD" "$TARGET" "$LABEL" "$EXP_FILE"
    done
}

# Builds and tests the client.
run_client_tests() {
    local TESTS=""

    cargo clean &> /dev/null

    build_client

    # Parse test parameters from the file names.
    TESTS=($( find "$CLIENT_TESTS_DIR" -type f -name "*.txt" -print0 | \
              xargs -0 -I {} basename --suffix ".txt" "{}" | \
              tr '\n' ' ' ))

    echo "CLIENT TESTS:"

    for test in "${TESTS[@]}"; do
        (( TOTAL_CLIENT_TESTS++ ))

        local METHOD=""
        local TARGET=""
        local LABEL=""

        local FILE="${test}.txt"
        local EXP_FILE="${CLIENT_TESTS_DIR}/${FILE}"

        METHOD="${test%_*}"
        METHOD="${METHOD^^}"

        TARGET="${test#*_}"
        TARGET="${TARGET,,}"

        case "$TARGET" in
        "jpeg")
            TARGET="/image/jpeg";;
        "png")
            TARGET="/image/png";;
        "svg")
            TARGET="/image/svg";;
        "text")
            TARGET="/robots.txt";;
        "utf8")
            TARGET="/encoding/utf8";;
        "webp")
            TARGET="/image/webp";;
        *)
            TARGET="/$TARGET";;
        esac

        LABEL="$METHOD $TARGET"
        run_one_client_test "$METHOD" "$TARGET" "$LABEL" "$EXP_FILE"
    done
}

# Prints the overall results to the terminal.
print_overall_results() {
    local C_TOTAL="$TOTAL_CLIENT_TESTS"
    local C_PASSED="$CLIENT_NUM_PASSED"

    local S_TOTAL="$TOTAL_SERVER_TESTS"
    local S_PASSED="$SERVER_NUM_PASSED"

    if (( ($S_TOTAL == 0) && ($C_TOTAL == 0) )); then
        return
    fi

    echo -e "\n${BLU}+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+${CLR}"

    if (( $C_TOTAL > 0 )); then
        echo -n "CLIENT: "

        if (( $C_TOTAL == $C_PASSED )); then
            echo -e "${GRN}${C_PASSED} of ${C_TOTAL} tests passed.${CLR}"
        else
            echo -e "${RED}${C_PASSED} of ${C_TOTAL} tests passed.${CLR}"
        fi
    fi

    if (( $S_TOTAL > 0 )); then
        echo -n "SERVER: "

        if (( $S_TOTAL == $S_PASSED )); then
            echo -e "${GRN}${S_PASSED} of ${S_TOTAL} tests passed.${CLR}"
        else
            echo -e "${RED}${S_PASSED} of ${S_TOTAL} tests passed.${CLR}"
        fi
    fi

    echo -e "${BLU}+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+${CLR}"
}

# Clean up build artifacts and testing debris.
clean_up() {
    if [[ ! -z "$SERVER_PID" ]]; then
        ps -p "$SERVER_PID" &> /dev/null

        if (( $? == 0 )); then
            kill -SIGTERM "$SERVER_PID" &> /dev/null
            wait -f "$SERVER_PID" &> /dev/null
        fi

        SERVER_PID=""
    fi

    cargo clean &> /dev/null
}

# Prints a help message to the terminal.
print_help() {
    echo -e "${GRN}USAGE${CLR}"
    echo -e "    $(basename "$0") <ARGUMENT>\n"
    echo -e "${GRN}ARGUMENTS${CLR}"
    echo -e "    client   Run all client tests only."
    echo -e "    server   Run all server tests only."
    echo -e "    all      Run all tests.\n"
}

# Handle command line arguments.
if (( $# < 1 )); then
    echo -e "${YLW}Please select a test group to run.${CLR}\n"
    print_help
else
    CLI_ARG="${1,,}"

    case "$CLI_ARG" in
    "client")
        run_client_tests
        print_overall_results
        clean_up;;
    "server")
        run_server_tests
        print_overall_results
        clean_up;;
    "all")
        run_client_tests
        echo
        run_server_tests
        print_overall_results
        clean_up;;
    *)
        echo -e "${YLW}Unknown argument: \"${1}\"${CLR}\n"
        print_help;;
    esac
fi

unset CRATE_DIR RED GRN BLU CYAN YLW PURP CLR TOTAL_SERVER_TESTS SERVER_TESTS_DIR
unset SERVER_NUM_PASSED SERVER_PID SERVER_BIN CLIENT_BIN CLIENT_TESTS_DIR
unset TOTAL_CLIENT_TESTS CLIENT_NUM_PASSED

exit
