#!/bin/bash

ADMIN_NAME="Admin"
ADMIN_PRINCIPAL=$(dfx identity get-principal)
ARENA_NAME="Neural Arena"
ARENA_PRINCIPAL=$(dfx canister id neuralarena_be)

dfx deploy brains_be --argument "(record {controllers=vec {record {name=\"$ADMIN_NAME\"; \"principal\"=principal \"$ADMIN_PRINCIPAL\"; description=null}; record {name=\"$ARENA_NAME\"; \"principal\"=principal \"$ARENA_PRINCIPAL\"; description=null}}})"