#!/bin/bash

ARENA_BE_ID=$(dfx canister id neuralarena_be)
BRAINS_FE_ID=$(dfx canister id brains_fe)

ADMIN_NAME="Admin"
ADMIN_PRINCIPAL=$(dfx identity get-principal)
ARENA_NAME="Neural Arena"
ARENA_PRINCIPAL=$ARENA_BE_ID
ASSET_CANISTER_URL="http://localhost:5173/{}?canisterId=$BRAINS_FE_ID"

dfx deploy brains_be --argument "(record {asset_canister_url=\"$ASSET_CANISTER_URL\"; controllers=vec {record {name=\"$ADMIN_NAME\"; \"principal\"=principal \"$ADMIN_PRINCIPAL\"; description=null}; record {name=\"$ARENA_NAME\"; \"principal\"=principal \"$ARENA_PRINCIPAL\"; description=null}}})"