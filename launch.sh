#!/usr/bin/env bash

PROJECT_NAME="uo2d"
KILL_SERVER=true

# Build the Rust project.
cargo build
build_status=$?

# Check if cargo build was successful.
if [ $build_status -ne 0 ]; then
    echo "Cargo build failed, stopping the script."
    exit 1
else
    echo -e "Successful build!\n"
fi

# Start the server.
./target/debug/${PROJECT_NAME} --server &
server_pid=$!

# Wait for 2 seconds for server to start up.
sleep 2

# Start the client.
./target/debug/${PROJECT_NAME}

# Optional: stop the server after the client exits.
if [ "${KILL_SERVER}" == "true" ]; then
    echo "Killing the server with PID: ${server_pid}"
    kill $server_pid
    wait $server_pid
else
    echo "Server is still running with PID: ${server_pid}"
    # Optional wait for server to exit before quitting script.
    # wait $server_pid
fi
