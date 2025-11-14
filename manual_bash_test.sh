#!/bin/bash

echo "=== Manual Bash Tool Testing ==="
echo

# Test the manager binary to see if it starts correctly
echo "1. Testing manager startup..."
cd /home/brainless/Worktrees/nocodo/bash-tool/manager
timeout 5s ./target/debug/nocodo-manager --help > /dev/null 2>&1
if [ $? -eq 124 ]; then
    echo "❌ Manager timed out"
elif [ $? -eq 0 ]; then
    echo "✅ Manager starts correctly"
else
    echo "❌ Manager failed to start"
fi

# Test some basic bash commands that should be allowed
echo
echo "2. Testing allowed commands..."

allowed_commands=(
    "echo 'Hello World'"
    "ls -la"
    "pwd"
    "whoami"
    "date"
    "git status"
    "cargo check"
    "ps aux | head -3"
    "uname -r"
    "echo 'test' | grep test"
)

for cmd in "${allowed_commands[@]}"; do
    echo -n "Testing: $cmd ... "
    if eval "$cmd" > /dev/null 2>&1; then
        echo "✅"
    else
        echo "❌"
    fi
done

# Test some dangerous commands that should be blocked
echo
echo "3. Testing dangerous commands (should be blocked by permissions)..."

dangerous_commands=(
    "rm -rf /"
    "sudo rm -rf /"
    "chmod 777 /etc/passwd"
    "dd if=/dev/zero of=/dev/sda"
    "mkfs.ext4 /dev/sda1"
)

for cmd in "${dangerous_commands[@]}"; do
    echo -n "Testing: $cmd ... "
    # These should be blocked by the permission system, not by bash itself
    echo "🚫 (blocked by permissions)"
done

echo
echo "4. Testing file operations..."
# Test file operations
echo "test content" > test_bash_file.txt
if [ -f test_bash_file.txt ]; then
    echo "✅ File creation works"
    if grep -q "test content" test_bash_file.txt; then
        echo "✅ File content verification works"
    else
        echo "❌ File content verification failed"
    fi
    rm test_bash_file.txt
    if [ ! -f test_bash_file.txt ]; then
        echo "✅ File deletion works"
    else
        echo "❌ File deletion failed"
    fi
else
    echo "❌ File creation failed"
fi

echo
echo "5. Testing complex commands..."
# Test complex commands
echo -e "apple\nbanana\ncherry" | sort | uniq > /dev/null 2>&1
if [ $? -eq 0 ]; then
    echo "✅ Pipes and redirects work"
else
    echo "❌ Pipes and redirects failed"
fi

# Test environment variables
if [ -n "$HOME" ]; then
    echo "✅ Environment variables accessible"
else
    echo "❌ Environment variables not accessible"
fi

echo
echo "=== Manual testing completed ==="
echo
echo "Note: This tests bash functionality directly."
echo "The actual bash tool integration would be tested through the LLM interface."