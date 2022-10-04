#!/bin/bash
set -o verbose
set -o errexit
cd web
basic-http-server . -a 127.0.0.1:1234
