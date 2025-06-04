#!/bin/bash

export DEBIAN_FRONTEND=noninteractive

apt update
apt install -y libclang-dev libssl-dev
