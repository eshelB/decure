#!/bin/bash

set -eu
set -o pipefail # If anything in a pipeline fails, the pipe's exit status is a failure
#set -x # Print all commands for debugging

# This means we don't need to configure the cli since it uses the preconfigured cli in the docker.
# We define this as a function rather than as an alias because it has more flexible expansion behavior.
# In particular, it's not possible to dynamically expand aliases, but `tx_of` dynamically executes whatever
# we specify in its arguments.
function secretcli() {
    set -e

    local container_name="mydev1"

    if [[ -z "${IS_GITHUB_ACTIONS+x}" ]]; then
      docker exec $container_name /usr/bin/secretd "$@"
    else
      SGX_MODE=SW /usr/local/bin/secretcli "$@"
    fi
}

# Just like `echo`, but prints to stderr
function log() {
    echo "$@" >&2
}

# suppress all output to stdout for the command described in the arguments
function quiet() {
    "$@" >/dev/null
}

# suppress all output to stdout and stderr for the command described in the arguments
function silent() {
    "$@" >/dev/null 2>&1
}

function assert_eq() {
    set -e
    local left="$1"
    local right="$2"
    local message

    if [[ "$left" != "$right" ]]; then
        if [ -z ${3+x} ]; then
            local lineno="${BASH_LINENO[0]}"
            log "assertion failed on line $lineno - both sides differ."
            log "left:"
            log "${left@Q}"
            log
            log "right:"
            log "${right@Q}"
        else
            message="$3"
            log "$message"
        fi
        return 1
    fi

    return 0
}

# Keep polling the blockchain until the tx completes.
# The first argument is the tx hash.
# The second argument is a message that will be logged after every failed attempt.
# The tx information will be returned.
function wait_for_tx() {
    local tx_hash="$1"
    local message="$2"

    local result

    log "waiting on tx: $tx_hash"
    # secretcli will only print to stdout when it succeeds
    until result="$(secretcli query tx "$tx_hash" 2>/dev/null)"; do
        log "$message"
        sleep 1
    done

    log "init complete"

    # log out-of-gas events
    if quiet jq -e '.raw_log | startswith("execute contract failed: Out of gas: ") or startswith("out of gas:")' <<<"$result"; then
        log "$(jq -r '.raw_log' <<<"$result")"
    fi

    log "finish wait"

    echo "$result"
}

# This is a wrapper around `wait_for_tx` that also decrypts the response,
# and returns a nonzero status code if the tx failed
function wait_for_compute_tx() {
    local tx_hash="$1"
    local message="$2"
    local return_value=0
    local result
    local decrypted

    result="$(wait_for_tx "$tx_hash" "$message")"
    # log "$result"
    if quiet jq -e '.logs == null' <<<"$result"; then
        return_value=1
    fi
    decrypted="$(secretcli query compute tx "$tx_hash")" || return
    log "$decrypted"
    echo "$decrypted"

    return "$return_value"
}

# If the tx failed, return a nonzero status code.
# The decrypted error or message will be echoed
function check_tx() {
    local tx_hash="$1"
    local result
    local return_value=0

    result="$(secretcli query tx "$tx_hash")"
    if quiet jq -e '.logs == null' <<<"$result"; then
        return_value=1
    fi
    decrypted="$(secretcli query compute tx "$tx_hash")" || return
    log "$decrypted"
    echo "$decrypted"

    return "$return_value"
}

# Extract the tx_hash from the output of the command
function tx_of() {
    "$@" | jq -r '.txhash'
}

# Extract the output_data_as_string from the output of the command
function data_of() {
    "$@" | jq -r '.output_data_as_string'
}

function get_generic_err() {
    jq -r '.output_error.generic_err.msg' <<<"$1"
}

# Send a compute transaction and return the tx hash.
# All arguments to this function are passed directly to `secretcli tx compute execute`.
function compute_execute() {
    tx_of secretcli tx compute execute "$@"
}

# Send a query to the contract.
# All arguments to this function are passed directly to `secretcli query compute query`.
function compute_query() {
    secretcli query compute query "$@"
}

# Generate a label for a contract with a given code id
# This just adds "contract_" before the code id.
function label_by_id() {
    local id="$1"
    echo "contract_$id"
}

function log_test_header() {
    log " ########### Starting ${FUNCNAME[1]} ###############################################################################################################################################"
}

function test_wrong_query_variant() {
    set -e
    local contract_addr="$1"
    log testing on contract "$contract_addr"

    log_test_header
    expected_error="Error: this is the expected error"

    result="$(compute_query "$contract_addr" '{"get_count":{}}' 2>&1 || true )"
    result_comparable=$(echo $result | sed 's/Usage:.*//' | awk '{$1=$1};1')

    assert_eq "$result_comparable" "$expected_error"
    log "wrong query variant: SUCCESS!"
}

function test_register_business_long_name() {
    set -e
    local contract_addr="$1"

    log_test_header
    expected_error="Error: this is the expected error"

    register_business_message='{"register_business":{"name":"AVeryLongNameForABusinessIsNotAccepted","description":"a place to eat","address":"address"}}'
    tx_hash="$(compute_execute "$contract_addr" "$register_business_message" --from a --gas 150000 -y)"
    # Notice the `!` before the command - it is EXPECTED to fail.
    ! register_business_response="$(wait_for_compute_tx "$tx_hash" "waiting for register business")"
    assert_eq \
        "$(get_generic_err "$register_business_response")" \
        "Name length can't be bigger than 20"

    log "register business long name: SUCCESS!"
}

function test_register_business_long_description() {
    set -e
    local contract_addr="$1"

    log_test_header
    expected_error="Error: this is the expected error"

    register_business_message='{"register_business":{"name":"shortName","description":"a place to eat with a very long description","address":"address"}}'
    tx_hash="$(compute_execute "$contract_addr" "$register_business_message" --from a --gas 150000 -y)"
    # Notice the `!` before the command - it is EXPECTED to fail.
    ! register_business_response="$(wait_for_compute_tx "$tx_hash" "waiting for register business")"
    assert_eq \
        "$(get_generic_err "$register_business_response")" \
        "Description length can't be bigger than 40"

    log "register business long description: SUCCESS!"
}

function main() {
    set -e
    log '              <####> Starting single-function test <####>'
    log "secretcli version in the docker image is: $(secretcli version)"

    local container_hash
    container_hash=$(docker ps | grep mydev | cut -d " " -f 1)
    log "the container hash is"
    log $container_hash

    local current_dir
    current_dir=$(pwd)

    log "getting last contract code"
    last_code=$(secretcli query compute list-code | jq ".[-1].id")
    log "last code is $last_code"
    last_address=$(secretcli query compute list-contract-by-code $last_code | jq ".[0].address" | tr -d '"')
    log "last address is $last_address"

    # this script should only be run from the project's root dir
    assert_eq "$current_dir" "/home/esh/Development/projects/decure"

    # test_wrong_query_variant "$last_address"
    # test_register_business_long_name "$last_address"
    test_register_business_long_description "$last_address"

    log 'test single func completed successfully'
    return 0
}

main "$@"
