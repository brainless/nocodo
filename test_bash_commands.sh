#!/bin/bash

# Test script for bash tool functionality
# This script tests various commands that should be allowed through the bash tool

echo "=== Testing Bash Tool Functionality ==="
echo

# Test 1: Basic echo command
echo "Test 1: Basic echo command"
echo 'Hello, World!'

# Test 2: List files
echo -e "\nTest 2: List files in current directory"
ls -la

# Test 3: Git commands
echo -e "\nTest 3: Git status"
git status

# Test 4: Cargo check
echo -e "\nTest 4: Cargo check"
cargo check

# Test 5: File operations
echo -e "\nTest 5: File operations"
echo "test content" > test_file.txt
cat test_file.txt
rm test_file.txt

# Test 6: Process information
echo -e "\nTest 6: Process information"
ps aux | head -5

# Test 7: Network connectivity (basic)
echo -e "\nTest 7: Network connectivity"
ping -c 1 8.8.8.8 || echo "Ping failed (expected in some environments)"

# Test 8: Environment variables
echo -e "\nTest 8: Environment variables"
echo "HOME: $HOME"
echo "PATH: ${PATH:0:50}..."

# Test 9: Text processing
echo -e "\nTest 9: Text processing"
echo -e "apple\nbanana\ncherry\napple" | sort | uniq

# Test 10: System information
echo -e "\nTest 10: System information"
uname -a

echo -e "\n=== All tests completed ==="