use std::fs;
use std::path::PathBuf;

use crate::domain::{Card, Subtask};

pub struct PlanGenerator;

impl PlanGenerator {
    pub fn generate_plan(card: &Card, subtasks: &[Subtask]) -> Result<String, String> {
        let linked_documents: Vec<String> = serde_json::from_str(&card.linked_documents)
            .map_err(|e| format!("Failed to parse linked_documents JSON: {}", e))?;

        let slug = Self::slugify(&card.title);
        let references = if linked_documents.is_empty() {
            "None".to_string()
        } else {
            linked_documents
                .iter()
                .map(|doc| format!("`{}`", doc))
                .collect::<Vec<_>>()
                .join(", ")
        };

        let mut output = String::new();
        output.push_str(&format!("# {}\n\n", card.title));
        output.push_str("## TL;DR\n");
        output.push_str(&format!("> {}\n", card.description));
        output.push_str(&format!(
            "> Deliverables: {} subtasks to complete\n\n",
            subtasks.len()
        ));

        output.push_str("## Context\n");
        output.push_str("### Card Details\n");
        output.push_str(&format!("- Priority: {}\n", card.priority));
        output.push_str(&format!("- Stage: {} (dispatched from todo)\n", card.stage));
        output.push_str(&format!(
            "- Working Directory: {}\n\n",
            card.working_directory
        ));

        output.push_str("### Referenced Documents\n");
        if linked_documents.is_empty() {
            output.push_str("- None\n\n");
        } else {
            for doc in &linked_documents {
                output.push_str(&format!("- `{}`\n", doc));
            }
            output.push('\n');
        }

        output.push_str("## TODOs\n\n");
        for (index, subtask) in subtasks.iter().enumerate() {
            let (category, skills) = Self::detect_agent_profile(&subtask.title);
            let skills_text = if skills.is_empty() {
                "[]".to_string()
            } else {
                format!(
                    "[{}]",
                    skills
                        .into_iter()
                        .map(|skill| format!("`{}`", skill))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };

            output.push_str(&format!("- [ ] {}. {}\n", index + 1, subtask.title));
            output.push_str(&format!("  **What to do**: {}\n", subtask.title));
            output.push_str("  **Recommended Agent Profile**:\n");
            output.push_str(&format!("  - Category: `{}`\n", category));
            output.push_str(&format!("  - Skills: {}\n", skills_text));
            output.push_str(&format!("  **References**: {}\n", references));
            output.push_str("  **Acceptance Criteria**:\n");
            output.push_str(&format!(
                "  - [ ] {} completed successfully\n",
                subtask.title
            ));
            output.push_str("  - [ ] Changes verified and tested\n");
            output.push_str("  **Commit**: YES\n");
            output.push_str(&format!(
                "  - Message: `{}({}): {}`\n\n",
                category, slug, subtask.title
            ));
        }

        Ok(output)
    }

    pub fn write_plan_file(
        working_directory: &str,
        card_title: &str,
        plan_content: &str,
    ) -> Result<String, String> {
        let mut plan_dir = PathBuf::from(working_directory);
        plan_dir.push(".sisyphus");
        plan_dir.push("plans");

        fs::create_dir_all(&plan_dir).map_err(|e| {
            format!(
                "Failed to create plan directory '{}': {}",
                plan_dir.display(),
                e
            )
        })?;

        let mut plan_path = plan_dir;
        plan_path.push(format!("{}.md", Self::slugify(card_title)));

        fs::write(&plan_path, plan_content)
            .map_err(|e| format!("Failed to write plan file '{}': {}", plan_path.display(), e))?;

        Ok(plan_path.to_string_lossy().to_string())
    }

    fn slugify(text: &str) -> String {
        let mut slug = String::new();
        let mut previous_dash = false;

        for ch in text.to_lowercase().chars() {
            if ch.is_ascii_alphanumeric() {
                slug.push(ch);
                previous_dash = false;
            } else if !previous_dash {
                slug.push('-');
                previous_dash = true;
            }
        }

        slug.trim_matches('-').to_string()
    }

    fn detect_agent_profile(subtask_title: &str) -> (&'static str, Vec<&'static str>) {
        let title = subtask_title.to_lowercase();

        if ["ui", "frontend", "component", "page", "style", "design"]
            .iter()
            .any(|keyword| title.contains(keyword))
        {
            return ("visual-engineering", vec!["frontend-ui-ux", "playwright"]);
        }

        if ["complex", "algorithm", "architecture", "optimization"]
            .iter()
            .any(|keyword| title.contains(keyword))
        {
            return ("ultrabrain", vec![]);
        }

        if ["bug", "fix", "typo", "rename"]
            .iter()
            .any(|keyword| title.contains(keyword))
        {
            return ("quick", vec![]);
        }

        if ["test", "spec"]
            .iter()
            .any(|keyword| title.contains(keyword))
        {
            return ("unspecified-high", vec![]);
        }

        ("unspecified-high", vec![])
    }
}
