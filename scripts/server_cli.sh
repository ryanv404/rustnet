#!/usr/bin/bash

./target/debug/server \
    --log \
    --debug-cli \
    --test-server \
    --file-route 'get:/:static/index.html' \
    --file-route 'get:/about:static/about.html' \
    --file-route 'head:/head:static/index.html' \
    --file-route 'post:/post:static/index.html' \
    --file-route 'put:/put:static/index.html' \
    --file-route 'patch:/patch:static/index.html' \
    --file-route 'delete:/delete:static/index.html' \
    --file-route 'trace:/trace:static/index.html' \
    --file-route 'options:/options:static/index.html' \
    --file-route 'connect:/connect:static/index.html' \
    --text-route 'get:/many_methods:Hi from the GET route!' \
    --text-route 'head:/many_methods:Hi from the HEAD route!' \
    --text-route 'post:/many_methods:Hi from the POST route!' \
    --text-route 'put:/many_methods:Hi from the PUT route!' \
    --text-route 'patch:/many_methods:Hi from the PATCH route!' \
    --text-route 'delete:/many_methods:Hi from the DELETE route!' \
    --text-route 'trace:/many_methods:Hi from the TRACE route!' \
    --text-route 'options:/many_methods:Hi from the OPTIONS route!' \
    --text-route 'connect:/many_methods:Hi from the CONNECT route!' \
    --text-route 'get:/text:hi everyone from this bash script!' \
    --favicon-route 'static/favicon.ico' \
    --not-found-route 'static/error.html' \
    localhost:7878
