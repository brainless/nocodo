#[cfg(test)]
mod tests {
    use crate::bash::bash_permissions::{BashPermissions, PermissionRule};
    use tempfile::TempDir;

    #[test]
    fn test_bash_permissions_default() {
        let perms = BashPermissions::default();

        // Test default allow behavior - many common commands are allowed by default
        assert!(perms.is_command_allowed("git status"));
        assert!(perms.is_command_allowed("ls -la"));
        assert!(perms.is_command_allowed("cargo build"));
        assert!(perms.is_command_allowed("npm test"));
        assert!(perms.is_command_allowed("echo hello"));
        
        // Test that dangerous commands are still blocked
        assert!(!perms.is_command_allowed("rm -rf /"));
        assert!(!perms.is_command_allowed("sudo rm -rf"));
    }

    #[test]
    fn test_bash_permissions_custom_rules() {
        let mut perms = BashPermissions::default();
        
        // Add allow rule for git commands
        let git_rule = PermissionRule::allow("git*")
            .expect("Failed to create git rule");
        perms.add_rule(git_rule);
        
        // Test that git commands are now allowed
        assert!(perms.is_command_allowed("git status"));
        assert!(perms.is_command_allowed("git commit"));
        assert!(perms.is_command_allowed("git push"));
        
        // Test that other commands are still blocked
        assert!(!perms.is_command_allowed("rm -rf /"));
        assert!(!perms.is_command_allowed("sudo rm -rf"));
    }

    #[test]
    fn test_bash_permissions_rule_management() {
        let mut perms = BashPermissions::default();
        let initial_count = perms.get_rules().len();
        
        // Add rule
        let rule = PermissionRule::allow("test*")
            .expect("Failed to create test rule");
        perms.add_rule(rule);
        
        // Check rule was added
        assert_eq!(perms.get_rules().len(), initial_count + 1);
        
        // Remove rule
        assert!(perms.remove_rule("test*"));
        assert_eq!(perms.get_rules().len(), initial_count);
        
        // Try to remove non-existent rule
        assert!(!perms.remove_rule("nonexistent*"));
    }

    #[test]
    fn test_bash_permissions_working_dir_management() {
        let temp_dir = TempDir::new().unwrap();
        let allowed_path = temp_dir.path().to_string_lossy().to_string();
        
        let mut perms = BashPermissions::default();
        // Add specific cd permission for the allowed path only
        let allowed_cd_cmd = format!("cd {}", allowed_path);
        perms.add_rule(PermissionRule::allow(&allowed_cd_cmd).unwrap());
        perms = perms.with_allowed_working_dirs(vec![allowed_path.clone()]);
        
        // Should allow commands in allowed directory
        assert!(perms.is_command_allowed(&allowed_cd_cmd));
        
        // Should block commands outside allowed directory
        assert!(!perms.is_command_allowed("cd /etc"));
        assert!(!perms.is_command_allowed("cd /root"));
    }
}