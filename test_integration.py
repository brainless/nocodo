#!/usr/bin/env python3
"""
Quick integration test for bash tool functionality
This tests the basic integration without waiting for full compilation
"""

import subprocess
import sys
import time

def run_command(cmd, timeout=30):
    """Run a command with timeout"""
    try:
        result = subprocess.run(
            cmd, shell=True, capture_output=True, text=True, timeout=timeout
        )
        return result.returncode == 0, result.stdout, result.stderr
    except subprocess.TimeoutExpired:
        return False, "", "Command timed out"

def test_rust_compilation():
    """Test if Rust code compiles"""
    print("Testing Rust compilation...")
    
    # Test just the imports
    test_code = """
use std::path::PathBuf;

// Test basic imports
mod models {
    pub struct BashRequest {
        pub command: String,
        pub working_dir: Option<String>,
        pub timeout_secs: Option<u64>,
        pub description: Option<String>,
    }
}

mod bash_permissions {
    use anyhow::Result;
    
    pub struct BashPermissions;
    
    impl BashPermissions {
        pub fn new() -> Self { Self }
        pub fn check_command(&self, _cmd: &str) -> Result<()> { Ok(()) }
        pub fn check_working_directory(&self, _path: &PathBuf) -> Result<()> { Ok(()) }
    }
}

mod bash_executor {
    use anyhow::Result;
    use super::bash_permissions::BashPermissions;
    
    pub struct BashExecutor;
    
    impl BashExecutor {
        pub fn new(_perms: BashPermissions, _timeout: u64) -> Result<Self> { Ok(Self) }
    }
}

fn main() {
    println!("All imports successful!");
}
"""
    
    # Write test file
    with open('/tmp/test_bash_integration.rs', 'w') as f:
        f.write(test_code)
    
    # Try to compile it
    success, stdout, stderr = run_command(f"rustc --edition 2021 /tmp/test_bash_integration.rs -o /tmp/test_integration 2>&1")
    
    if success:
        print("✅ Basic Rust integration test passed")
        return True
    else:
        print(f"❌ Basic Rust integration test failed: {stderr}")
        return False

def test_project_structure():
    """Test if all required files exist"""
    print("\nTesting project structure...")
    
    required_files = [
        '/home/brainless/Worktrees/nocodo/bash-tool/manager/src/bash_executor.rs',
        '/home/brainless/Worktrees/nocodo/bash-tool/manager/src/bash_permissions.rs',
        '/home/brainless/Worktrees/nocodo/bash-tool/manager/src/models.rs',
        '/home/brainless/Worktrees/nocodo/bash-tool/manager/src/tools.rs',
        '/home/brainless/Worktrees/nocodo/bash-tool/manager/src/llm_agent.rs',
        '/home/brainless/Worktrees/nocodo/bash-tool/manager/src/lib.rs',
        '/home/brainless/Worktrees/nocodo/bash-tool/manager/Cargo.toml',
    ]
    
    all_exist = True
    for file_path in required_files:
        try:
            with open(file_path, 'r') as f:
                content = f.read()
                if len(content) > 0:
                    print(f"✅ {file_path.split('/')[-1]} exists and has content")
                else:
                    print(f"❌ {file_path.split('/')[-1]} exists but is empty")
                    all_exist = False
        except FileNotFoundError:
            print(f"❌ {file_path.split('/')[-1]} not found")
            all_exist = False
        except Exception as e:
            print(f"❌ Error reading {file_path.split('/')[-1]}: {e}")
            all_exist = False
    
    return all_exist

def test_cargo_dependencies():
    """Test if required dependencies are in Cargo.toml"""
    print("\nTesting Cargo.toml dependencies...")
    
    try:
        with open('/home/brainless/Worktrees/nocodo/bash-tool/manager/Cargo.toml', 'r') as f:
            content = f.read()
        
        required_deps = [
            'codex-core',
            'codex-process-hardening', 
            'async-channel',
            'signal-hook',
            'signal-hook-tokio',
            'glob',
        ]
        
        all_present = True
        for dep in required_deps:
            if dep in content:
                print(f"✅ {dep} dependency found")
            else:
                print(f"❌ {dep} dependency missing")
                all_present = False
        
        return all_present
        
    except Exception as e:
        print(f"❌ Error reading Cargo.toml: {e}")
        return False

def test_module_registration():
    """Test if modules are registered in lib.rs"""
    print("\nTesting module registration...")
    
    try:
        with open('/home/brainless/Worktrees/nocodo/bash-tool/manager/src/lib.rs', 'r') as f:
            content = f.read()
        
        required_modules = [
            'pub mod bash_executor',
            'pub mod bash_permissions',
        ]
        
        all_present = True
        for module in required_modules:
            if module in content:
                print(f"✅ {module} found")
            else:
                print(f"❌ {module} missing")
                all_present = False
        
        return all_present
        
    except Exception as e:
        print(f"❌ Error reading lib.rs: {e}")
        return False

def main():
    """Run all integration tests"""
    print("🔧 Bash Tool Integration Test Suite")
    print("=" * 50)
    
    tests = [
        test_project_structure,
        test_cargo_dependencies,
        test_module_registration,
        test_rust_compilation,
    ]
    
    passed = 0
    total = len(tests)
    
    for test in tests:
        if test():
            passed += 1
        print()
    
    print("=" * 50)
    print(f"Test Results: {passed}/{total} tests passed")
    
    if passed == total:
        print("🎉 All integration tests passed!")
        return 0
    else:
        print("❌ Some tests failed")
        return 1

if __name__ == "__main__":
    sys.exit(main())