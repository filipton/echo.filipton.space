#!/bin/bash

./build.sh

docker build -t filipton/echo .
docker push filipton/echo:latest
