#!/bin/bash
# Run script for Linux
echo "Building monitor-daemon..."
cargo build --release

if [ $? -eq 0 ]; then
    echo "Starting monitor-daemon..."
    ./target/release/monitor-daemon
else
    echo "Build failed. Please check the errors above."
fi
