#!/bin/bash

set -e # Fail the whole script on first error

ssh -o StrictHostKeyChecking=accept-new git@github.com

git submodule update --init --recursive
