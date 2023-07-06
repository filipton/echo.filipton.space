#!/bin/bash

cwd=$(pwd)

mkdir -p ./build/public

cd ./frontend
npm run build
cd $cwd

rm -rf ./build/public/*
cp -r ./frontend/build/* ./build/public
cd ./backend
cargo build --release
cd $cwd

cp ./backend/target/release/backend ./build/backend
