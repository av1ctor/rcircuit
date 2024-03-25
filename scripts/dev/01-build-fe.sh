#!/bin/bash
PWD=$(pwd)
TARGET=$PWD/src/rcircuit_fe/assets/wasm
cd ../rcircuit-engine && npm run build-wasm && cp ./pkg/game-engine_bg.wasm $TARGET && cd $PWD