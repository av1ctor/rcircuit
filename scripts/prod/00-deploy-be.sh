#!/bin/bash

dfx identity use deployer

ARENA_BE_ID=$(dfx canister id neuralarena_be --network ic)
RCIRCUIT_FE_ID=$(dfx canister id rcircuit_fe --network ic)

ADMIN_NAME="Admin"
ADMIN_PRINCIPAL=$(dfx identity get-principal)
ARENA_NAME="Neural Arena"
ARENA_PRINCIPAL=$ARENA_BE_ID
ASSET_CANISTER_URL="https://$RCIRCUIT_FE_ID.icp0.io/{}"

dfx deploy -v --network ic --identity deployer --with-cycles 1000000000000 rcircuit_be --argument "'(record {
    asset_canister_url=\"$ASSET_CANISTER_URL\"; 
    controllers=vec {
        record {
            name=\"$ADMIN_NAME\"; 
            \"principal\"=principal \"$ADMIN_PRINCIPAL\"; 
            description=null
        }; 
        record {
            name=\"$ARENA_NAME\"; 
            \"principal\"=principal \"$ARENA_PRINCIPAL\"; 
            description=null
        }
    }
})'"

dfx identity use default