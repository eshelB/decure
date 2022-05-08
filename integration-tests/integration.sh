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

    # log "waiting on tx: $tx_hash"
    # secretcli will only print to stdout when it succeeds
    until result="$(secretcli query tx "$tx_hash" 2>/dev/null)"; do
        log "$message"
        sleep 1
    done

    # log "init complete"

    # log out-of-gas events
    if quiet jq -e '.raw_log | startswith("execute contract failed: Out of gas: ") or startswith("out of gas:")' <<<"$result"; then
        log "$(jq -r '.raw_log' <<<"$result")"
    fi

    # log "finish wait"

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
    # log "$decrypted"
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
    # log getting tx hash
    local result
    result=$("$@")
    # log the result is
    # log "$result"
    echo "$result" | jq -r '.txhash'
}

# Extract the output_data_as_string from the output of the command
function data_of() {
    "$@" | jq -r '.output_data_as_string'
}

function get_generic_err() {
    jq -r '.output_error.generic_err.msg' <<< "$1"
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

function upload_code() {
    set -e
    local directory="$1"
    local tx_hash
    local code_id

    log uploading code from dir "$directory"

    tx_hash="$(tx_of secretcli tx compute store "$directory/contract.wasm.gz" --from a -y --gas 10000000)"
    log "uploaded contract with tx hash $tx_hash"
    code_id="$(
        wait_for_tx "$tx_hash" 'waiting for contract upload' |
            jq -r '.logs[0].events[0].attributes[] | select(.key == "code_id") | .value'
    )"

    log "uploaded contract #$code_id"

    echo "$code_id"
}

# Generate a label for a contract with a given code id
# This just adds "contract_" before the code id.
function label_by_id() {
    local id="$1"
    echo "contract_$id"
}

function instantiate() {
    set -e
    local code_id="$1"
    local init_msg="$2"

    log 'sending init message:'
    log "${init_msg}"

    local tx_hash
    tx_hash="$(tx_of secretcli tx compute instantiate "$code_id" "$init_msg" --label "$(label_by_id "$code_id")" --from a --gas "10000000" -y)"
    wait_for_tx "$tx_hash" 'waiting for init to complete'
    log "instantiation completed"
}

# This function uploads and instantiates a contract, and returns the new contract's address
function create_contract() {
    set -e
    local dir="$1"
    local init_msg="$2"

    local code_id
    code_id="$(upload_code "$dir")"

    local init_result
    init_result="$(instantiate "$code_id" "$init_msg")"

    if quiet jq -e '.logs == null' <<<"$init_result"; then
        local tx_hash
        tx_hash=$(jq -r '.txhash' <<<"$init_result")
        log "$(secretcli query compute tx "$tx_hash")"
        return 1
    fi

    # log init result "$init_result"
    result=$(jq -r '.logs[0].events[0].attributes[] | select(.key == "contract_address") | .value' <<<"$init_result")

    log "contract address is $result"
    echo "$result"
}

function log_test_header() {
    log
    log "########### Starting ${FUNCNAME[1]} ####################################################################################################################################"
}

function test_register_business() {
    set -e
    local contract_addr="$1"
    local business_addr="$2"

    log_test_header

    register_business_message='{"register_business":{"name":"Starbucks","description":"a place to eat","address":"'"$business_addr"'"}}'
    log message "$(jq <<< "$register_business_message")"
    tx_hash="$(compute_execute "$contract_addr" "$register_business_message" --from a --gas 150000 -y)"
    register_business_result="$(data_of wait_for_compute_tx "$tx_hash" 'waiting for register_business from "a" to process')"
    log result "$(jq <<< "$register_business_result")"
    local status
    status=$(jq -er '.register_business.status' <<< "$register_business_result")

    assert_eq "$status" "successfully called register business"

    log "register business: SUCCESS!"
}

function test_register_existing_business() {
    set -e
    local contract_addr="$1"
    local business_addr="$2"

    log_test_header

    register_business_message='{"register_business":{"name":"Starbucks","description":"a place to eat","address":"'"$business_addr"'"}}'
    log message "$(jq <<< "$register_business_message")"
    tx_hash="$(compute_execute "$contract_addr" "$register_business_message" --from a --gas 150000 -y)"
    local register_business_result
    ! register_business_result="$(wait_for_compute_tx "$tx_hash" 'waiting for register_business')"

    assert_eq \
        "$(get_generic_err "$register_business_result")" \
        "A business is already registered on that address"

    log "register existing business: SUCCESS!"
}

function test_review_tx_bad_recipient () {
    set -e
    local contract_addr="$1"
    local business_addr="$2"

    log_test_header

    local review_message
    # note that this tx_id belongs to "a" but it is sent to "c" (b is the business)
    review_message='{
      "review_business": {
        "address":"'"$business_addr"'",
        "content":"unexistent tx",
        "rating":0,
        "title":"better going somewhere else",
        "tx_id": 8,
        "tx_page": 0,
        "viewing_key": "vk"
      }
    }'

    log message "$(jq '.output_error' <<< "$review_message")"
    tx_hash="$(compute_execute "$contract_addr" "$review_message" --from a --gas 150000 -y)"
    local review_message_result
    ! review_message_result="$(wait_for_compute_tx "$tx_hash" 'waiting for review business with unexistent tx')"
    log result "$(jq '.output_error' <<< "$review_message_result")"

    assert_eq \
        "$(get_generic_err "$review_message_result")" \
        "The specified transfer's recipient is not the specified business"

    log "review business with unexistent TX: SUCCESS!"
}

function test_review_unexistent_tx () {
    set -e
    local contract_addr="$1"
    local business_addr="$2"

    log_test_header

    local review_message
    # note that this tx_id belongs to "d" and not to "c"
    review_message='{
      "review_business": {
        "address":"'"$business_addr"'",
        "content":"unexistent tx",
        "rating":0,
        "title":"better going somewhere else",
        "tx_id": 7,
        "tx_page": 0,
        "viewing_key": "vk"
      }
    }'

    log message "$(jq '.output_error' <<< "$review_message")"
    tx_hash="$(compute_execute "$contract_addr" "$review_message" --from a --gas 150000 -y)"
    local review_message_result
    ! review_message_result="$(wait_for_compute_tx "$tx_hash" 'waiting for review business with unexistent tx')"
    log result "$(jq '.output_error' <<< "$review_message_result")"

    assert_eq \
        "$(get_generic_err "$review_message_result")" \
        "there was no transaction with id 7 in the specified page"

    log "review business with unexistent TX: SUCCESS!"
}

function test_reviews () {
    set -e
    local contract_addr="$1"
    local business_addr="$2"

    log_test_header

    local review_message
    review_message='{
      "review_business": {
        "address": "'"$business_addr"'",
        "content": "great stuff!",
        "rating": 5,
        "title":"amazing restaurant",
        "tx_id": 4,
        "tx_page": 0,
        "viewing_key": "vk"
      }
    }'

    tx_hash="$(compute_execute "$contract_addr" "$review_message" --from a --gas 150000 -y)"
    review_business_result="$(data_of wait_for_compute_tx "$tx_hash" 'waiting for rating by a')"
    log result "$(jq <<< "$review_business_result")"
    local status
    status=$(jq -er '.review_business.status' <<< "$review_business_result")

    assert_eq "$status" "Successfully added a new review on business, receipt was accounted for"
    log "review business: SUCCESS!"

    local query_single_business_message
    query_single_business_message='{
      "get_single_business": {
        "address":"'"$business_addr"'"
      }
    }'

    result="$(compute_query "$contract_addr" "$query_single_business_message" 2>&1 || true )"
    result_comparable=$(echo $result | sed 's/ Usage:.*//')
    assert_eq "$result_comparable" '{"single_business":{"business":{"name":"Starbucks","description":"a place to eat","address":"secret1fc3fzy78ttp0lwuujw7e52rhspxn8uj52zfyne","average_rating":"5000","reviews_count":1},"status":"Successfully retrieved business by address"}}'
    local rating
    rating="$(jq -er '.single_business.business.average_rating' <<< "$result_comparable")"
    log "rating after a rated: $rating"
    assert_eq $rating '5000'

    local review_message
    review_message='{
      "review_business": {
        "address": "'"$business_addr"'",
        "content": "Not so good",
        "rating": 0,
        "title": "D first review",
        "tx_id": 5,
        "tx_page": 0,
        "viewing_key": "vk"
      }
    }'

    log message "$(jq <<< "$review_message")"
    tx_hash="$(compute_execute "$contract_addr" "$review_message" --from d --gas 150000 -y)"
    local review_business_result
    review_business_result="$(data_of wait_for_compute_tx "$tx_hash" "waiting for rating by d")"
    log result "$(jq <<< "$review_business_result")"
    local status
    status=$(jq -er '.review_business.status' <<< "$review_business_result")

    assert_eq "$status" "Successfully added a new review on business, receipt was accounted for"

    local query_single_business_message
    query_single_business_message='{
      "get_single_business": {
        "address":"'"$business_addr"'"
      }
    }'

    result="$(compute_query "$contract_addr" "$query_single_business_message" 2>&1 || true )"
    result_comparable=$(echo $result | sed 's/ Usage:.*//')
    local rating
    rating="$(jq -er '.single_business.business.average_rating' <<< "$result_comparable")"
    log "rating after d rated: $rating"
    assert_eq $rating '1250'
    log "query single business: SUCCESS!"

    local review_message
    review_message='{
      "review_business": {
        "address": "'"$business_addr"'",
        "content": "In the middle",
        "rating": 3,
        "title": "not good and not bad",
        "tx_id": 6,
        "tx_page": 0,
        "viewing_key": "vk"
      }
    }'

    log message "$(jq <<< "$review_message")"
    tx_hash="$(compute_execute "$contract_addr" "$review_message" --from c --gas 150000 -y)"
    review_business_result="$(data_of wait_for_compute_tx "$tx_hash" 'waiting for rating by c')"
    log result "$(jq <<< "$review_business_result")"
    local status
    status=$(jq -er '.review_business.status' <<< "$review_business_result")

    assert_eq "$status" "Successfully added a new review on business, receipt was accounted for"

    local query_single_business_message
    query_single_business_message='{
      "get_single_business": {
        "address":"'"$business_addr"'"
      }
    }'

    log query message "$(jq <<< "$query_single_business_message")"
    result="$(compute_query "$contract_addr" "$query_single_business_message" 2>&1 || true )"
    result_comparable=$(echo $result | sed 's/ Usage:.*//')
    local rating
    rating="$(jq -er '.single_business.business.average_rating' <<< "$result_comparable")"
    log "rating after c rated: $rating"
    assert_eq $rating '1833'

    local query_reviews_on_business
    query_reviews_on_business='{
      "get_reviews_on_business": {
        "business_address":"'"$business_addr"'",
        "page_size": 8
      }
    }'

    log query message "$(jq <<< "$query_reviews_on_business")"
    result="$(compute_query "$contract_addr" "$query_reviews_on_business" 2>&1 || true )"
    result_comparable=$(echo $result | sed 's/ Usage:.*//')
    local reviews
    reviews="$(jq -er '' <<< "$result_comparable")"
    log "all reviews after c rated: $reviews"

    local review_message
    review_message='{
      "review_business": {
        "address": "'"$business_addr"'",
        "content": "second time was amazing",
        "rating": 5,
        "title": "2nd time is the charm",
        "tx_id": 7,
        "tx_page": 0,
        "viewing_key": "vk"
      }
    }'

    log message "$(jq <<< "$review_message")"
    tx_hash="$(compute_execute "$contract_addr" "$review_message" --from d --gas 150000 -y)"
    review_business_result="$(data_of wait_for_compute_tx "$tx_hash" 'waiting for 2nd rating by d')"
    log result "$(jq <<< "$review_business_result")"
    local status
    status=$(jq -er '.review_business.status' <<< "$review_business_result")

    assert_eq "$status" "Successfully updated a previous review on business, receipt was accounted for"

    local query_single_business_message
    query_single_business_message='{
      "get_single_business": {
        "address":"'"$business_addr"'"
      }
    }'

    log query message "$(jq <<< "$query_single_business_message")"
    result="$(compute_query "$contract_addr" "$query_single_business_message" 2>&1 || true )"
    result_comparable=$(echo $result | sed 's/ Usage:.*//')
    local rating
    rating="$(jq -er '.single_business.business.average_rating' <<< "$result_comparable")"
    log "rating after 2nd time d rated: $rating"
    assert_eq $rating '4428'

    local query_reviews_on_business
    query_reviews_on_business='{
      "get_reviews_on_business": {
        "business_address":"'"$business_addr"'",
        "page_size": 8
      }
    }'

    log query message "$(jq <<< "$query_reviews_on_business")"
    result="$(compute_query "$contract_addr" "$query_reviews_on_business" 2>&1 || true )"
    result_comparable=$(echo $result | sed 's/ Usage:.*//')
    local reviews
    reviews="$(jq -er '' <<< "$result_comparable")"
    log "all reviews after d rated for 2nd time: $reviews"

    local query_all_businesses
    query_all_businesses='{
      "get_businesses": {
        "page_size": 8
      }
    }'

    log query message "$(jq <<< "$query_all_businesses")"
    result="$(compute_query "$contract_addr" "$query_all_businesses" 2>&1 || true )"
    result_comparable=$(echo $result | sed 's/ Usage:.*//')
    local businesses
    businesses="$(jq -er '' <<< "$result_comparable")"
    log "all businesses query result: $businesses"

    log "query single business: SUCCESS!"
}

function test_register_business_long_name() {
    set -e
    local contract_addr="$1"

    log_test_header

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
    log '              <####> Starting local deploy <####>'
    log "secretcli version in the docker image is: $(secretcli version)"
    secretcli config output json

    # build optimized contract
    log "building contract in optimizer docker"
    docker run --rm -v "$(pwd)":/contract \
    --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
    --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
    enigmampc/secret-contract-optimizer

    local optimizer_result
    optimizer_result="$?"
    log "built optimized contract"
    log "the result of the optimization is: $optimizer_result"


    local dir
    if [[ -z "${IS_GITHUB_ACTIONS+x}" ]]; then
      local container_hash
      container_hash=$(docker ps | grep mydev | cut -d " " -f 1)
      log "the container hash is $container_hash"
      docker cp ./contract.wasm.gz "$container_hash":/root/code
      dir="code"
    else
      dir="$(pwd)"
    fi

    log "copied contract wasm to container"

    local init_msg
    init_msg='{}'
    contract_addr="$(create_contract "$dir" "$init_msg")"

    local business_address
    business_address="secret1fc3fzy78ttp0lwuujw7e52rhspxn8uj52zfyne"

    log
    log "################ DEMO ####################"
    test_register_business "$contract_addr" "$business_address"
    test_reviews "$contract_addr" "$business_address"

    log
    log "########### VALIDATIONS ##################"
    test_review_unexistent_tx "$contract_addr" "$business_address"
    test_register_existing_business "$contract_addr" "$business_address"
    test_register_business_long_name "$contract_addr"
    test_register_business_long_description "$contract_addr"
    test_review_tx_bad_recipient "$contract_addr" "$business_address"

    log 'deploy + test completed successfully'

    return 0
}

main "$@"
