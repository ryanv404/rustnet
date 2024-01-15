#!/usr/bin/bash

./target/debug/client \
    --debug \
    --plain \
    --no-dates \
    --output 'Rshb' \
    --method 'put' \
    --header 'content-length:13' \
    --header 'content-type:text/plain' \
    --body 'Test message.' \
    'httpbin.org/status/200'
