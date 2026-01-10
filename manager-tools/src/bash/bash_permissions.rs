use anyhow::{anyhow, Result};
use glob::Pattern;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{debug, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionAction {
    Allow,
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRule {
    pub pattern: String,
    pub action: PermissionAction,
    pub description: Option<String>,
    #[serde(skip)]
    compiled_pattern: Pattern,
}

impl PermissionRule {
    pub fn allow(pattern: &str) -> Result<Self> {
        Ok(Self {
            pattern: pattern.to_string(),
            action: PermissionAction::Allow,
            description: None,
            compiled_pattern: Pattern::new(pattern)
                .map_err(|e| anyhow!("Invalid glob pattern '{}': {}", pattern, e))?,
        })
    }

    pub fn deny(pattern: &str) -> Result<Self> {
        Ok(Self {
            pattern: pattern.to_string(),
            action: PermissionAction::Deny,
            description: None,
            compiled_pattern: Pattern::new(pattern)
                .map_err(|e| anyhow!("Invalid glob pattern '{}': {}", pattern, e))?,
        })
    }

    pub fn with_description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    pub fn matches(&self, command: &str) -> bool {
        self.compiled_pattern.matches(command)
    }

    pub fn is_allowed(&self) -> bool {
        matches!(self.action, PermissionAction::Allow)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BashPermissions {
    rules: Vec<PermissionRule>,
    default_action: PermissionAction,
    allowed_working_dirs: Vec<String>,
    deny_changing_to_sensitive_dirs: bool,
}

#[allow(dead_code)]
impl BashPermissions {
    pub fn new(rules: Vec<PermissionRule>) -> Self {
        Self {
            rules,
            default_action: PermissionAction::Deny,
            allowed_working_dirs: vec![
                "/tmp".to_string(),
                "/home".to_string(),
                "/workspace".to_string(),
                "/project".to_string(),
            ],
            deny_changing_to_sensitive_dirs: true,
        }
    }

    pub fn with_default_action(mut self, action: PermissionAction) -> Self {
        self.default_action = action;
        self
    }

    pub fn with_allowed_working_dirs(mut self, dirs: Vec<String>) -> Self {
        self.allowed_working_dirs = dirs;
        self
    }

    pub fn with_sensitive_dir_protection(mut self, enabled: bool) -> Self {
        self.deny_changing_to_sensitive_dirs = enabled;
        self
    }

    pub fn check_command(&self, command: &str) -> Result<()> {
        debug!("Checking command permissions: {}", command);

        // Check each rule in order - first match wins
        for rule in &self.rules {
            if rule.matches(command) {
                if rule.is_allowed() {
                    debug!("Command allowed by rule: {}", rule.pattern);
                    return Ok(());
                } else {
                    warn!("Command denied by rule: {}", rule.pattern);
                    return Err(anyhow!(
                        "Command denied by rule: {}{}",
                        rule.pattern,
                        rule.description
                            .as_ref()
                            .map(|d| format!(" ({})", d))
                            .unwrap_or_default()
                    ));
                }
            }
        }

        // If no rules matched, use default action
        match self.default_action {
            PermissionAction::Allow => {
                debug!("Command allowed by default action");
                Ok(())
            }
            PermissionAction::Deny => {
                warn!("Command denied by default action");
                Err(anyhow!("Command denied by default policy"))
            }
        }
    }

    pub fn is_command_allowed(&self, command: &str) -> bool {
        self.check_command(command).is_ok()
    }

    pub fn check_working_directory(&self, working_dir: &Path) -> Result<()> {
        let working_dir_str = working_dir.to_string_lossy();
        debug!(
            "Checking working directory permissions: {}",
            working_dir_str
        );

        // Check if directory is in allowed list
        let is_allowed = self
            .allowed_working_dirs
            .iter()
            .any(|allowed| working_dir_str.starts_with(allowed) || working_dir_str == *allowed);

        if !is_allowed {
            warn!("Working directory not in allowed list: {}", working_dir_str);
            return Err(anyhow!(
                "Working directory '{}' not in allowed list",
                working_dir_str
            ));
        }

        // Check for sensitive directories
        if self.deny_changing_to_sensitive_dirs {
            let sensitive_dirs = [
                "/etc", "/boot", "/sys", "/proc", "/dev", "/root", "/var/run", "/var/log",
            ];

            for sensitive in &sensitive_dirs {
                if working_dir_str.starts_with(sensitive) {
                    warn!("Access to sensitive directory denied: {}", working_dir_str);
                    return Err(anyhow!(
                        "Access to sensitive directory '{}' is denied",
                        working_dir_str
                    ));
                }
            }
        }

        debug!("Working directory allowed: {}", working_dir_str);
        Ok(())
    }

    pub fn add_rule(&mut self, rule: PermissionRule) {
        self.rules.push(rule);
    }

    pub fn remove_rule(&mut self, pattern: &str) -> bool {
        let initial_len = self.rules.len();
        self.rules.retain(|rule| rule.pattern != pattern);
        self.rules.len() != initial_len
    }

    pub fn get_rules(&self) -> &[PermissionRule] {
        &self.rules
    }

    pub fn get_allowed_working_dirs(&self) -> &[String] {
        &self.allowed_working_dirs
    }

    pub fn add_allowed_working_dir(&mut self, dir: String) {
        if !self.allowed_working_dirs.contains(&dir) {
            self.allowed_working_dirs.push(dir);
        }
    }

    pub fn remove_allowed_working_dir(&mut self, dir: &str) -> bool {
        let initial_len = self.allowed_working_dirs.len();
        self.allowed_working_dirs.retain(|d| d != dir);
        self.allowed_working_dirs.len() != initial_len
    }

    /// Create permissions that only allow specific commands
    ///
    /// # Arguments
    /// * `commands` - List of command patterns to allow (e.g., "tesseract*")
    ///
    /// # Examples
    /// ```rust
    /// let perms = BashPermissions::only_allow(vec!["tesseract*", "ls*"]);
    /// ```
    pub fn only_allow(commands: Vec<&str>) -> Self {
        let mut perms = Self::new(vec![]);

        // Add allow rules for specified commands
        for cmd in commands {
            perms.add_rule(PermissionRule::allow(cmd).unwrap());
        }

        // Deny everything else
        perms.add_rule(PermissionRule::deny("*").unwrap());

        perms
    }

    /// Create read-only permissions (ls, cat, grep, etc.)
    pub fn read_only() -> Self {
        let mut perms = Self::new(vec![]);

        perms.add_rule(PermissionRule::allow("ls*").unwrap());
        perms.add_rule(PermissionRule::allow("cat*").unwrap());
        perms.add_rule(PermissionRule::allow("head*").unwrap());
        perms.add_rule(PermissionRule::allow("tail*").unwrap());
        perms.add_rule(PermissionRule::allow("grep*").unwrap());
        perms.add_rule(PermissionRule::allow("find*").unwrap());
        perms.add_rule(PermissionRule::allow("wc*").unwrap());
        perms.add_rule(PermissionRule::allow("pwd").unwrap());

        perms.add_rule(PermissionRule::deny("*").unwrap());

        perms
    }

    /// Create minimal permissions (only the specified command, no utilities)
    ///
    /// This is the most restrictive option - only the exact command(s) specified
    ///
    /// # Examples
    /// ```rust
    /// // Only allow tesseract command
    /// let perms = BashPermissions::minimal(vec!["tesseract"]);
    /// ```
    pub fn minimal(commands: Vec<&str>) -> Self {
        let mut perms = Self::new(vec![]);

        for cmd in commands {
            // Allow exact command and with arguments
            let pattern = format!("{}*", cmd);
            perms.add_rule(PermissionRule::allow(&pattern).unwrap());
        }

        // Deny everything else
        perms.add_rule(PermissionRule::deny("*").unwrap());

        perms
    }
}

impl Default for BashPermissions {
    fn default() -> Self {
        let rules = vec![
            PermissionRule::allow("echo*")
                .unwrap()
                .with_description("Allow echo commands"),
            PermissionRule::allow("ls*")
                .unwrap()
                .with_description("Allow listing files"),
            PermissionRule::allow("cat*")
                .unwrap()
                .with_description("Allow reading files"),
            PermissionRule::allow("pwd")
                .unwrap()
                .with_description("Allow showing current directory"),
            PermissionRule::allow("which*")
                .unwrap()
                .with_description("Allow finding commands"),
            PermissionRule::allow("git status")
                .unwrap()
                .with_description("Allow git status"),
            PermissionRule::allow("git add*")
                .unwrap()
                .with_description("Allow git add"),
            PermissionRule::allow("git commit*")
                .unwrap()
                .with_description("Allow git commit"),
            PermissionRule::allow("git log*")
                .unwrap()
                .with_description("Allow git log"),
            PermissionRule::allow("git diff*")
                .unwrap()
                .with_description("Allow git diff"),
            PermissionRule::allow("git show*")
                .unwrap()
                .with_description("Allow git show"),
            PermissionRule::allow("cargo check")
                .unwrap()
                .with_description("Allow cargo check"),
            PermissionRule::allow("cargo test")
                .unwrap()
                .with_description("Allow cargo test"),
            PermissionRule::allow("cargo build*")
                .unwrap()
                .with_description("Allow cargo build"),
            PermissionRule::allow("npm test")
                .unwrap()
                .with_description("Allow npm test"),
            PermissionRule::allow("npm install")
                .unwrap()
                .with_description("Allow npm install"),
            PermissionRule::allow("npm run*")
                .unwrap()
                .with_description("Allow npm run commands"),
            PermissionRule::allow("python*")
                .unwrap()
                .with_description("Allow python commands"),
            PermissionRule::allow("make*")
                .unwrap()
                .with_description("Allow make commands"),
            PermissionRule::allow("find*")
                .unwrap()
                .with_description("Allow finding files"),
            PermissionRule::allow("grep*")
                .unwrap()
                .with_description("Allow grep search"),
            PermissionRule::allow("head*")
                .unwrap()
                .with_description("Allow head command"),
            PermissionRule::allow("tail*")
                .unwrap()
                .with_description("Allow tail command"),
            PermissionRule::allow("wc*")
                .unwrap()
                .with_description("Allow word count"),
            PermissionRule::allow("sort*")
                .unwrap()
                .with_description("Allow sort"),
            PermissionRule::allow("uniq*")
                .unwrap()
                .with_description("Allow uniq"),
            PermissionRule::deny("rm -rf /*")
                .unwrap()
                .with_description("Prevent catastrophic deletion"),
            PermissionRule::deny("rm -rf /")
                .unwrap()
                .with_description("Prevent root deletion"),
            PermissionRule::deny("chmod 777 /*")
                .unwrap()
                .with_description("Prevent global permission changes"),
            PermissionRule::deny("chmod 777 /")
                .unwrap()
                .with_description("Prevent root permission changes"),
            PermissionRule::deny("sudo *")
                .unwrap()
                .with_description("Prevent sudo usage"),
            PermissionRule::deny("su *")
                .unwrap()
                .with_description("Prevent su usage"),
            PermissionRule::deny("passwd*")
                .unwrap()
                .with_description("Prevent password changes"),
        ];

        Self::new(rules)
            .with_default_action(PermissionAction::Deny)
            .with_sensitive_dir_protection(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_only_allow() {
        let perms = BashPermissions::only_allow(vec!["tesseract*", "ls*"]);

        // Should allow specified commands
        assert!(perms.check_command("tesseract input.png output").is_ok());
        assert!(perms.check_command("ls -la").is_ok());

        // Should deny others
        assert!(perms.check_command("cat file.txt").is_err());
        assert!(perms.check_command("rm file.txt").is_err());
    }

    #[test]
    fn test_read_only() {
        let perms = BashPermissions::read_only();

        // Should allow read commands
        assert!(perms.check_command("ls -la").is_ok());
        assert!(perms.check_command("cat file.txt").is_ok());
        assert!(perms.check_command("grep pattern file.txt").is_ok());

        // Should deny write commands
        assert!(perms.check_command("echo test > file.txt").is_err());
        assert!(perms.check_command("rm file.txt").is_err());
    }

    #[test]
    fn test_minimal() {
        let perms = BashPermissions::minimal(vec!["tesseract"]);

        // Should allow only specified command
        assert!(perms.check_command("tesseract input.png output").is_ok());

        // Should deny everything else, including safe commands
        assert!(perms.check_command("ls -la").is_err());
        assert!(perms.check_command("cat file.txt").is_err());
    }

    #[test]
    fn test_permission_rule_creation() {
        let rule = PermissionRule::allow("echo*").unwrap();
        assert!(rule.is_allowed());
        assert!(rule.matches("echo hello"));
        assert!(!rule.matches("cat file"));

        let deny_rule = PermissionRule::deny("rm*").unwrap();
        assert!(!deny_rule.is_allowed());
        assert!(deny_rule.matches("rm file"));
    }

    #[test]
    fn test_bash_permissions_allow() {
        let permissions = BashPermissions::new(vec![
            PermissionRule::allow("echo*").unwrap(),
            PermissionRule::allow("ls*").unwrap(),
        ]);

        assert!(permissions.check_command("echo hello").is_ok());
        assert!(permissions.check_command("ls -la").is_ok());
        assert!(permissions.check_command("cat file").is_err());
    }

    #[test]
    fn test_bash_permissions_deny() {
        let permissions = BashPermissions::new(vec![
            PermissionRule::allow("echo*").unwrap(),
            PermissionRule::deny("rm*").unwrap(),
        ]);

        assert!(permissions.check_command("echo hello").is_ok());
        assert!(permissions.check_command("rm file").is_err());
    }

    #[test]
    fn test_working_directory_permissions() {
        let permissions = BashPermissions::default();

        assert!(permissions
            .check_working_directory(Path::new("/tmp"))
            .is_ok());
        assert!(permissions
            .check_working_directory(Path::new("/home/user"))
            .is_ok());
        assert!(permissions
            .check_working_directory(Path::new("/etc"))
            .is_err());
        assert!(permissions
            .check_working_directory(Path::new("/root"))
            .is_err());
    }

    #[test]
    fn test_default_permissions() {
        let permissions = BashPermissions::default();

        // Should allow safe commands
        assert!(permissions.check_command("echo hello").is_ok());
        assert!(permissions.check_command("ls -la").is_ok());
        assert!(permissions.check_command("git status").is_ok());
        assert!(permissions.check_command("cargo check").is_ok());

        // Should deny dangerous commands
        assert!(permissions.check_command("rm -rf /").is_err());
        assert!(permissions.check_command("sudo rm -rf /").is_err());
        assert!(permissions.check_command("passwd").is_err());
    }

    #[test]
    fn test_bash_permissions_custom_rules() {
        let mut perms = BashPermissions::default();

        // Add custom allow rule
        let allow_rule = PermissionRule::allow("make*")
            .unwrap()
            .with_description("Allow make commands");
        perms.add_rule(allow_rule);

        // Add custom deny rule
        let deny_rule = PermissionRule::deny("docker*")
            .unwrap()
            .with_description("Block docker commands");
        perms.add_rule(deny_rule);

        assert!(perms.is_command_allowed("make build"));
        assert!(!perms.is_command_allowed("docker run ubuntu"));
    }

    #[test]
    fn test_bash_permissions_working_directory() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let allowed_path = temp_dir.path().to_string_lossy().to_string();

        let perms = BashPermissions::new(vec![PermissionRule::allow("cd*").unwrap()])
            .with_allowed_working_dirs(vec![allowed_path.clone()]);

        // Should allow commands in allowed directory
        let allowed_cmd = format!("cd {}", allowed_path);
        assert!(perms.is_command_allowed(&allowed_cmd));

        // Check working directory permissions directly
        assert!(perms.check_working_directory(temp_dir.path()).is_ok());
        assert!(perms.check_working_directory(Path::new("/etc")).is_err());
        assert!(perms.check_working_directory(Path::new("/root")).is_err());
    }

    #[test]
    fn test_bash_permissions_rule_management() {
        let mut perms = BashPermissions::new(vec![]);

        // Add rule
        let rule = PermissionRule::allow("test*")
            .unwrap()
            .with_description("Test rule");
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
        let mut perms = BashPermissions::new(vec![]).with_allowed_working_dirs(vec![]);

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

        // Test exact matches (using commands that are actually allowed)
        assert!(perms.is_command_allowed("git status"));
        assert!(perms.is_command_allowed("ls"));
        assert!(perms.is_command_allowed("cargo build"));

        // Test partial matches (should not match)
        assert!(perms.is_command_allowed("git status")); // This is actually safe
        assert!(perms.is_command_allowed("ls -l")); // This is actually safe

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
            assert!(
                !perms.is_command_allowed(cmd),
                "Command should be blocked: {}",
                cmd
            );
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
            assert!(
                perms.is_command_allowed(cmd),
                "Command should be allowed: {}",
                cmd
            );
        }
    }
}
