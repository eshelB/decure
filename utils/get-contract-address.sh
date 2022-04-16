#!/bin/bash

set -eu
set -o pipefail # If anything in a pipeline fails, the pipe's exit status is a failure
#set -x # Print all commands for debugging

# This means we don't need to configure the cli since it uses the preconfigured cli in the docker.
# We define this as a function rather than as an alias because it has more flexible expansion behavior.
# In particular, it's not possible to dynamically expand aliases, but `tx_of` dynamically executes whatever
# we specify in its arguments.
function secretcli() {
    SGX_MODE=SW /usr/local/bin/secretcli "$@"
}

# Just like `echo`, but prints to stderr
function log() {
    echo "$@" >&2
}

function main() {
    set -e
    log '########## Getting last deployed contract address ########'

    local last_code

    log "getting last contract code"
    last_code=$(secretcli query compute list-code | tail -n 5 | grep id | cut -d " " -f 6 | tr "," " ")
    log "last code is $last_code"
    last_address=$(secretcli query compute list-contract-by-code $last_code | jq ".[0].address")
    log "last address is $last_address"

    return 0
}

main "$@"
