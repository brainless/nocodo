#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::path::PathBuf;

    #[test]
    fn test_permission_rule_creation() {
        let rule = PermissionRule {
            pattern: glob::Pattern::new("git*").unwrap(),
            allow: true,
            description: "Allow git commands".to_string(),
        };

        assert!(rule.allow);
        assert_eq!(rule.description, "Allow git commands");
        assert!(rule.pattern.matches("git status"));
        assert!(rule.pattern.matches("git commit"));
        assert!(!rule.pattern.matches("rm -rf"));
    }

    #[test]
    fn test_bash_permissions_default() {
        let perms = BashPermissions::default();
        
        // Should allow safe commands by default
        assert!(perms.is_command_allowed("git status"));
        assert!(perms.is_command_allowed("ls -la"));
        assert!(perms.is_command_allowed("cargo build"));
        assert!(perms.is_command_allowed("npm test"));
        
        // Should block dangerous commands
        assert!(!perms.is_command_allowed("rm -rf /"));
        assert!(!perms.is_command_allowed("sudo rm -rf"));
        assert!(!perms.is_command_allowed("chmod 777 /etc/passwd"));
        assert!(!perms.is_command_allowed("dd if=/dev/zero of=/dev/sda"));
    }

    #[test]
    fn test_bash_permissions_custom_rules() {
        let mut perms = BashPermissions::default();
        
        // Add custom allow rule
        let allow_rule = PermissionRule {
            pattern: glob::Pattern::new("make*").unwrap(),
            allow: true,
            description: "Allow make commands".to_string(),
        };
        perms.add_rule(allow_rule);
        
        // Add custom deny rule
        let deny_rule = PermissionRule {
            pattern: glob::Pattern::new("docker*").unwrap(),
            allow: false,
            description: "Block docker commands".to_string(),
        };
        perms.add_rule(deny_rule);
        
        assert!(perms.is_command_allowed("make build"));
        assert!(!perms.is_command_allowed("docker run ubuntu"));
    }

    #[test]
    fn test_bash_permissions_working_directory() {
        let temp_dir = TempDir::new().unwrap();
        let allowed_path = temp_dir.path().to_string_lossy().to_string();
        
        let perms = BashPermissions::default()
            .with_allowed_working_dirs(vec![allowed_path.clone()]);
        
        // Should allow commands in allowed directory
        let allowed_cmd = format!("cd {}", allowed_path);
        assert!(perms.is_command_allowed(&allowed_cmd));
        
        // Should block commands outside allowed directory
        assert!(!perms.is_command_allowed("cd /etc"));
        assert!(!perms.is_command_allowed("cd /root"));
    }

    #[test]
    fn test_bash_permissions_rule_management() {
        let mut perms = BashPermissions::default();
        
        // Add rule
        let rule = PermissionRule {
            pattern: glob::Pattern::new("test*").unwrap(),
            allow: true,
            description: "Test rule".to_string(),
        };
        perms.add_rule(rule);
        
        // Check rule was added
        assert_eq!(perms.get_rules().len(), 1);
        
        // Remove rule
        assert!(perms.remove_rule("test*"));
        assert_eq!(perms.get_rules().len(), 0);
        
        // Try to remove non-existent rule
        assert!(!perms.remove_rule("nonexistent*"));
    }

    #[test]
    fn test_bash_permissions_working_dir_management() {
        let mut perms = BashPermissions::default();
        
        // Add allowed directory
        let dir1 = "/tmp/test1".to_string();
        let dir2 = "/tmp/test2".to_string();
        
        perms.add_allowed_working_dir(dir1.clone());
        perms.add_allowed_working_dir(dir2.clone());
        
        assert_eq!(perms.get_allowed_working_dirs().len(), 2);
        assert!(perms.get_allowed_working_dirs().contains(&dir1));
        assert!(perms.get_allowed_working_dirs().contains(&dir2));
        
        // Remove allowed directory
        assert!(perms.remove_allowed_working_dir(&dir1));
        assert_eq!(perms.get_allowed_working_dirs().len(), 1);
        assert!(!perms.get_allowed_working_dirs().contains(&dir1));
        assert!(perms.get_allowed_working_dirs().contains(&dir2));
        
        // Try to remove non-existent directory
        assert!(!perms.remove_allowed_working_dir("/nonexistent"));
    }

    #[test]
    fn test_command_sanitization() {
        let perms = BashPermissions::default();
        
        // Test command sanitization
        assert!(perms.is_command_allowed("echo hello"));
        assert!(perms.is_command_allowed("echo 'hello world'"));
        assert!(perms.is_command_allowed("echo \"hello world\""));
        
        // Test dangerous command patterns
        assert!(!perms.is_command_allowed("rm -rf /"));
        assert!(!perms.is_command_allowed("sudo rm -rf /"));
        assert!(!perms.is_command_allowed("chmod 777 /etc/passwd"));
        assert!(!perms.is_command_allowed("dd if=/dev/zero of=/dev/sda"));
        assert!(!perms.is_command_allowed("mkfs.ext4 /dev/sda1"));
        assert!(!perms.is_command_allowed("fdisk /dev/sda"));
    }

    #[test]
    fn test_pattern_matching_edge_cases() {
        let perms = BashPermissions::default();
        
        // Test exact matches
        assert!(perms.is_command_allowed("git"));
        assert!(perms.is_command_allowed("ls"));
        assert!(perms.is_command_allowed("cargo"));
        
        // Test partial matches (should not match)
        assert!(perms.is_command_allowed("git-status")); // This is actually safe
        assert!(perms.is_command_allowed("ls-l")); // This is actually safe
        
        // Test complex patterns
        assert!(perms.is_command_allowed("cargo build --release"));
        assert!(perms.is_command_allowed("npm run test"));
        assert!(perms.is_command_allowed("python -m pytest"));
    }

    #[test]
    fn test_default_deny_patterns() {
        let perms = BashPermissions::default();
        
        // Test that dangerous patterns are blocked by default
        let dangerous_commands = vec![
            "rm -rf /",
            "sudo rm -rf /",
            "chmod 777 /etc/passwd",
            "dd if=/dev/zero of=/dev/sda",
            "mkfs.ext4 /dev/sda1",
            "fdisk /dev/sda",
            "format c:",
            "del /s /q c:\\*.*",
        ];
        
        for cmd in dangerous_commands {
            assert!(!perms.is_command_allowed(cmd), "Command should be blocked: {}", cmd);
        }
    }

    #[test]
    fn test_default_allow_patterns() {
        let perms = BashPermissions::default();
        
        // Test that safe patterns are allowed by default
        let safe_commands = vec![
            "git status",
            "git add .",
            "git commit -m 'test'",
            "ls -la",
            "cargo build",
            "cargo test",
            "npm install",
            "npm test",
            "python -m pytest",
            "make build",
            "make test",
            "cat README.md",
            "grep -r \"pattern\" src/",
            "find . -name \"*.rs\"",
        ];
        
        for cmd in safe_commands {
            assert!(perms.is_command_allowed(cmd), "Command should be allowed: {}", cmd);
        }
    }
}