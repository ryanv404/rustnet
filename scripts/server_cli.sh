#!/usr/bin/bash

./target/debug/server \
    --log \
    --debug \
    --test \
    --file 'get:/:static/index.html' \
    --file 'get:/about:static/about.html' \
    --file 'head:/head:static/index.html' \
    --file 'post:/post:static/index.html' \
    --file 'put:/put:static/index.html' \
    --file 'patch:/patch:static/index.html' \
    --file 'delete:/delete:static/index.html' \
    --file 'trace:/trace:static/index.html' \
    --file 'options:/options:static/index.html' \
    --file 'connect:/connect:static/index.html' \
    --text 'get:/many_methods:Hi from the GET route!' \
    --text 'head:/many_methods:Hi from the HEAD route!' \
    --text 'post:/many_methods:Hi from the POST route!' \
    --text 'put:/many_methods:Hi from the PUT route!' \
    --text 'patch:/many_methods:Hi from the PATCH route!' \
    --text 'delete:/many_methods:Hi from the DELETE route!' \
    --text 'trace:/many_methods:Hi from the TRACE route!' \
    --text 'options:/many_methods:Hi from the OPTIONS route!' \
    --text 'connect:/many_methods:Hi from the CONNECT route!' \
    --text 'get:/text:hi everyone from this bash script!' \
    --favicon 'static/favicon.ico' \
    --not-found 'static/error.html' \
    localhost:7878
