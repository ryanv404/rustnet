#!/usr/bin/bash

./target/debug/server \
    -l \
    -d \
    -t \
    -f './log_file.txt' \
    -F 'get:/:static/index.html' \
    -F 'get:/about:static/about.html' \
    -F 'head:/head:static/index.html' \
    -F 'post:/post:static/index.html' \
    -F 'put:/put:static/index.html' \
    -F 'patch:/patch:static/index.html' \
    -F 'delete:/delete:static/index.html' \
    -F 'trace:/trace:static/index.html' \
    -F 'options:/options:static/index.html' \
    -F 'connect:/connect:static/index.html' \
    -T 'get:/many_methods:Hi from the GET route!' \
    -T 'head:/many_methods:Hi from the HEAD route!' \
    -T 'post:/many_methods:Hi from the POST route!' \
    -T 'put:/many_methods:Hi from the PUT route!' \
    -T 'patch:/many_methods:Hi from the PATCH route!' \
    -T 'delete:/many_methods:Hi from the DELETE route!' \
    -T 'trace:/many_methods:Hi from the TRACE route!' \
    -T 'options:/many_methods:Hi from the OPTIONS route!' \
    -T 'connect:/many_methods:Hi from the CONNECT route!' \
    -T 'get:/text:hi everyone from this bash script!' \
    -I 'static/favicon.ico' \
    -0 'static/error.html' \
    'localhost:7878'
