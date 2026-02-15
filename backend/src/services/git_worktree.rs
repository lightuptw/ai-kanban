use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::domain::KanbanError;

#[derive(Debug, Serialize, Deserialize)]
pub struct FileDiff {
    pub path: String,
    pub status: String,
    pub additions: i64,
    pub deletions: i64,
    pub diff: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiffResult {
    pub files: Vec<FileDiff>,
    pub stats: DiffStats,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiffStats {
    pub files_changed: i64,
    pub additions: i64,
    pub deletions: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MergeResult {
    pub success: bool,
    pub message: String,
    pub conflicts: Vec<String>,
}

pub struct GitWorktreeService;

impl GitWorktreeService {
    pub fn create_worktree(
        repo_path: &str,
        card_id: &str,
        card_title: &str,
    ) -> Result<(String, String), KanbanError> {
        let repo_root = Path::new(repo_path);
        if !repo_root.join(".git").exists() {
            return Err(KanbanError::BadRequest(format!(
                "Not a git repository: {}",
                repo_path
            )));
        }

        let slug = Self::slugify_title(card_title);
        let id_prefix: String = card_id.chars().take(8).collect();
        let branch_name = if slug.is_empty() {
            format!("ai/{}", id_prefix)
        } else {
            format!("ai/{}-{}", id_prefix, slug)
        };

        let worktree_root = repo_root.join(".lightup-workspaces");
        let worktree_path = worktree_root.join(card_id);

        Self::ensure_gitignore_entry(repo_path)?;

        let worktree_path_str = worktree_path.to_string_lossy().to_string();
        Self::run_git(
            repo_path,
            &[
                "worktree",
                "add",
                worktree_path_str.as_str(),
                "-b",
                branch_name.as_str(),
            ],
        )?;

        Ok((branch_name, worktree_path_str))
    }

    pub fn remove_worktree(
        repo_path: &str,
        worktree_path: &str,
        branch_name: &str,
    ) -> Result<(), KanbanError> {
        if !worktree_path.is_empty() {
            if let Err(error) =
                Self::run_git(repo_path, &["worktree", "remove", worktree_path, "--force"])
            {
                tracing::warn!(
                    repo_path,
                    worktree_path,
                    error = %error,
                    "Failed to remove git worktree"
                );
            }
        }

        if !branch_name.is_empty() {
            if let Err(error) = Self::run_git(repo_path, &["branch", "-D", branch_name]) {
                tracing::warn!(
                    repo_path,
                    branch_name,
                    error = %error,
                    "Failed to delete git branch during worktree cleanup"
                );
            }
        }

        Ok(())
    }

    pub fn get_diff(repo_path: &str, branch_name: &str) -> Result<DiffResult, KanbanError> {
        let default_branch = Self::detect_default_branch(repo_path);
        let range = format!("{}...{}", default_branch, branch_name);

        let name_status = Self::run_git(repo_path, &["diff", range.as_str(), "--name-status"])?;
        let numstat = Self::run_git(repo_path, &["diff", range.as_str(), "--numstat"])?;

        let mut stat_map: HashMap<String, (i64, i64)> = HashMap::new();
        for line in numstat.lines().filter(|line| !line.trim().is_empty()) {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() < 3 {
                continue;
            }

            let additions = parts[0].parse::<i64>().unwrap_or(0);
            let deletions = parts[1].parse::<i64>().unwrap_or(0);
            let path = parts.last().unwrap_or(&"").to_string();
            stat_map.insert(path, (additions, deletions));
        }

        let mut files = Vec::new();
        let mut total_additions = 0_i64;
        let mut total_deletions = 0_i64;

        for line in name_status.lines().filter(|line| !line.trim().is_empty()) {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() < 2 {
                continue;
            }

            let raw_status = parts[0];
            let status = if raw_status.starts_with('A') {
                "added"
            } else if raw_status.starts_with('M') {
                "modified"
            } else if raw_status.starts_with('D') {
                "deleted"
            } else if raw_status.starts_with('R') {
                "renamed"
            } else {
                "modified"
            }
            .to_string();

            let path = if raw_status.starts_with('R') && parts.len() >= 3 {
                parts[2].to_string()
            } else {
                parts[1].to_string()
            };

            let (additions, deletions) = stat_map.get(&path).copied().unwrap_or((0, 0));
            let diff = Self::run_git(repo_path, &["diff", range.as_str(), "--", path.as_str()])?;

            total_additions += additions;
            total_deletions += deletions;

            files.push(FileDiff {
                path,
                status,
                additions,
                deletions,
                diff,
            });
        }

        Ok(DiffResult {
            stats: DiffStats {
                files_changed: files.len() as i64,
                additions: total_additions,
                deletions: total_deletions,
            },
            files,
        })
    }

    pub fn merge_branch(repo_path: &str, branch_name: &str) -> Result<MergeResult, KanbanError> {
        let default_branch = Self::detect_default_branch(repo_path);

        let _previous_branch =
            Self::run_git(repo_path, &["rev-parse", "--abbrev-ref", "HEAD"]).ok();
        Self::run_git(repo_path, &["checkout", default_branch.as_str()])?;

        let merge_message = format!("Merge {}", branch_name);
        let merge_result = match Self::run_git(
            repo_path,
            &[
                "merge",
                branch_name,
                "--no-ff",
                "-m",
                merge_message.as_str(),
            ],
        ) {
            Ok(_) => MergeResult {
                success: true,
                message: format!("Merged {} into {}", branch_name, default_branch),
                conflicts: Vec::new(),
            },
            Err(error) => {
                let conflicts =
                    Self::run_git(repo_path, &["diff", "--name-only", "--diff-filter=U"])
                        .ok()
                        .map(|output| {
                            output
                                .lines()
                                .filter(|line| !line.trim().is_empty())
                                .map(str::to_owned)
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();

                let _ = Self::run_git(repo_path, &["merge", "--abort"]);

                MergeResult {
                    success: false,
                    message: format!("Merge failed: {}", error),
                    conflicts,
                }
            }
        };

        if let Err(error) = Self::run_git(repo_path, &["checkout", "-"]) {
            tracing::warn!(error = %error, "Failed to return to previous branch after merge");
        }

        Ok(merge_result)
    }

    pub fn create_github_pr(
        repo_path: &str,
        branch_name: &str,
        title: &str,
        body: &str,
    ) -> Result<String, KanbanError> {
        let default_branch = Self::detect_default_branch(repo_path);

        Self::run_git(repo_path, &["push", "origin", branch_name])?;

        let output = Command::new("gh")
            .args([
                "pr",
                "create",
                "--title",
                title,
                "--body",
                body,
                "--base",
                default_branch.as_str(),
                "--head",
                branch_name,
            ])
            .current_dir(repo_path)
            .output()
            .map_err(|e| KanbanError::Internal(format!("Failed to run gh: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(KanbanError::Internal(format!(
                "GitHub PR creation failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if stdout.is_empty() {
            return Err(KanbanError::Internal(
                "GitHub PR creation returned empty output".to_string(),
            ));
        }

        Ok(stdout)
    }

    fn ensure_gitignore_entry(repo_path: &str) -> Result<(), KanbanError> {
        let gitignore_path = Path::new(repo_path).join(".gitignore");
        let entry = ".lightup-workspaces/";

        let existing = if gitignore_path.exists() {
            fs::read_to_string(&gitignore_path)
                .map_err(|e| KanbanError::Internal(format!("Failed to read .gitignore: {}", e)))?
        } else {
            String::new()
        };

        let has_entry = existing
            .lines()
            .map(str::trim)
            .any(|line| line == entry || line == ".lightup-workspaces");

        if has_entry {
            return Ok(());
        }

        let mut updated = existing;
        if !updated.is_empty() && !updated.ends_with('\n') {
            updated.push('\n');
        }
        updated.push_str(entry);
        updated.push('\n');

        fs::write(&gitignore_path, updated)
            .map_err(|e| KanbanError::Internal(format!("Failed to update .gitignore: {}", e)))?;

        Ok(())
    }

    fn slugify_title(title: &str) -> String {
        let mut slug = String::new();
        let mut last_dash = false;

        for ch in title.to_lowercase().chars() {
            if ch.is_ascii_alphanumeric() {
                slug.push(ch);
                last_dash = false;
                continue;
            }

            if !last_dash {
                slug.push('-');
                last_dash = true;
            }
        }

        let mut trimmed = slug.trim_matches('-').to_string();
        if trimmed.len() > 40 {
            trimmed.truncate(40);
            trimmed = trimmed.trim_end_matches('-').to_string();
        }

        trimmed
    }

    fn detect_default_branch(repo_path: &str) -> String {
        if let Ok(output) = Self::run_git(repo_path, &["symbolic-ref", "refs/remotes/origin/HEAD"])
        {
            let branch_ref = output.trim();
            if let Some(branch) = branch_ref.rsplit('/').next() {
                if !branch.is_empty() {
                    return branch.to_string();
                }
            }
        }

        if Self::run_git(repo_path, &["rev-parse", "--verify", "main"]).is_ok() {
            return "main".to_string();
        }

        if Self::run_git(repo_path, &["rev-parse", "--verify", "master"]).is_ok() {
            return "master".to_string();
        }

        "main".to_string()
    }

    fn run_git(repo_path: &str, args: &[&str]) -> Result<String, KanbanError> {
        let output = Command::new("git")
            .args(args)
            .current_dir(repo_path)
            .output()
            .map_err(|e| KanbanError::Internal(format!("Failed to run git: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(KanbanError::Internal(format!(
                "Git command failed: {}",
                stderr.trim()
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}
