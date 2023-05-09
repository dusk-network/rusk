#!/bin/bash

arg_list=(
    asyncify-import@env.compute_proof_and_propagate
    asyncify-import@env.request_stct_proof
    asyncify-import@env.request_wfct_proof
    asyncify-import@env.fetch_anchor
    asyncify-import@env.fetch_stake
    asyncify-import@env.fetch_notes
    asyncify-import@env.fetch_existing_nullifiers
    asyncify-import@env.fetch_opening
)

printf -v args '%s,' "${arg_list[@]}"

wasm-opt --asyncify -O4 \
    --pass-arg "$args" \
    target/wasm32-unknown-unknown/release/dusk_wallet_core.wasm \
    -o mod.wasm
