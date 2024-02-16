#!/bin/bash
PWD=$(pwd)
TARGET=$PWD/src/brains_fe/assets/wasm
cd ../brains-engine && npm run build-wasm && cp ./pkg/game-engine_bg.wasm $TARGET && cd $PWD