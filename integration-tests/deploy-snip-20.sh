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

    # local res
    if [[ -z "${IS_GITHUB_ACTIONS+x}" ]]; then
      docker exec $container_name /usr/bin/secretd "$@"
    else
      SGX_MODE=SW /usr/local/bin/secretcli "$@"
    fi

    # echo "$res"
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

    log "waiting for tx: $tx_hash"
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

    result=$(jq -r '.logs[0].events[0].attributes[] | select(.key == "contract_address") | .value' <<<"$init_result")

    log "contract address is $result"
    echo "$result"
}

function main() {
    set -e
    log '              <####> Starting local deploy <####>'
    log "secretcli version in the docker image is: $(secretcli version)"

    local container_hash
    container_hash=$(docker ps | grep mydev | cut -d " " -f 1)

    local current_dir
    current_dir=$(pwd)

    # this script should only be run from the project's root dir
    assert_eq "$current_dir" "/home/esh/Development/projects/decure"

    # build optimized contract
    log "building contract in optimizer docker"

    docker cp /home/esh/WebstormProjects/public-clones/snip20/contract.wasm.gz "$container_hash":/root/code
    log "copied contract wasm to container"

    local prng_seed
    prng_seed="$(base64 <<<'enigma-rocks')"
    local init_msg
    init_msg='{"name":"secret-secret","admin":"secret1ap26qrlp8mcq2pg6r47w43l0y8zkqm8a450s03","symbol":"SSCRT","decimals":6,"initial_balances":[],"prng_seed":"'"$prng_seed"'","config":{"public_total_supply":true,"enable_deposit":true,"enable_redeem":true,"enable_mint":true,"enable_burn":true}}'
    dir="code"
    contract_addr="$(create_contract "$dir" "$init_msg")"

    log 'snip20 initialization completed successfully'
    local tx_hash

    log 'sending some scrt to c and d since they have none'
    tx_hash=$(tx_of secretcli tx bank send a secret1ldjxljw7v4vk6zhyduywh04hpj0jdwxsmrlatf 20uscrt -y)
    wait_for_tx "$tx_hash" "waiting for transfer to d"
    tx_hash=$(tx_of secretcli tx bank send a secret1ajz54hz8azwuy34qwy9fkjnfcrvf0dzswy0lqq 20uscrt -y)
    wait_for_tx "$tx_hash" "waiting for transfer to c"

    log 'setting viewing key to vk'
    secretcli tx snip20 set-viewing-key secret18vd8fpwxzck93qlwghaj6arh4p7c5n8978vsyg --from a vk -y
    secretcli tx snip20 set-viewing-key secret18vd8fpwxzck93qlwghaj6arh4p7c5n8978vsyg --from b vk -y
    secretcli tx snip20 set-viewing-key secret18vd8fpwxzck93qlwghaj6arh4p7c5n8978vsyg --from c vk -y
    tx_hash=$(tx_of secretcli tx snip20 set-viewing-key secret18vd8fpwxzck93qlwghaj6arh4p7c5n8978vsyg --from d vk -y)
    wait_for_tx "$tx_hash" "waiting for viewing key setting"

    log 'depositing some sscrt for a, c, and d'
    secretcli tx snip20 deposit secret18vd8fpwxzck93qlwghaj6arh4p7c5n8978vsyg --amount 10uscrt --from a -y
    secretcli tx snip20 deposit secret18vd8fpwxzck93qlwghaj6arh4p7c5n8978vsyg --amount 10uscrt --from c -y
    tx_hash=$(tx_of secretcli tx snip20 deposit secret18vd8fpwxzck93qlwghaj6arh4p7c5n8978vsyg --amount 10uscrt --from d -y)
    wait_for_tx "$tx_hash" "waiting for deposits"

    log 'creating 4 receipts and en extra tx for validation'
    log '1) a -> 1sscrt -> b'
    log '2) c -> 2sscrt -> b'
    log '3) d -> 3sscrt -> b'
    log '4) d -> 1 more sscrt -> b'
    log '5) a -> 1 -> c'


    secretcli tx snip20 send secret18vd8fpwxzck93qlwghaj6arh4p7c5n8978vsyg secret1fc3fzy78ttp0lwuujw7e52rhspxn8uj52zfyne 1 --from a -y
    secretcli tx snip20 send secret18vd8fpwxzck93qlwghaj6arh4p7c5n8978vsyg secret1fc3fzy78ttp0lwuujw7e52rhspxn8uj52zfyne 3 --from d -y
    tx_hash=$(tx_of secretcli tx snip20 send secret18vd8fpwxzck93qlwghaj6arh4p7c5n8978vsyg secret1fc3fzy78ttp0lwuujw7e52rhspxn8uj52zfyne 2 --from c -y)
    wait_for_tx "$tx_hash" "waiting for transfers a and c and d1"
    tx_hash=$(tx_of secretcli tx snip20 send secret18vd8fpwxzck93qlwghaj6arh4p7c5n8978vsyg secret1fc3fzy78ttp0lwuujw7e52rhspxn8uj52zfyne 1 --from d -y)
    secretcli tx snip20 send secret18vd8fpwxzck93qlwghaj6arh4p7c5n8978vsyg secret1ajz54hz8azwuy34qwy9fkjnfcrvf0dzswy0lqq 3 --from a -y
    wait_for_tx "$tx_hash" "waiting for transfer d2 and a2"

    log 'transactions sent to b:'
    secretcli q snip20 transfers secret18vd8fpwxzck93qlwghaj6arh4p7c5n8978vsyg secret1fc3fzy78ttp0lwuujw7e52rhspxn8uj52zfyne vk 0 | jq
}

main "$@"
