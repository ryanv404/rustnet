#!/usr/bin/bash

./target/debug/client \
    -d \
    -n \
    -p \
    -O "Rshb" \
    -M "put" \
    -P "/json" \
    -H "content-length:13" \
    -H "content-type:text/plain" \
    -B "Test message." \
    'httpbin.org'
