use axum::Json;
use serde::Serialize;
use std::process::Command;

use crate::domain::KanbanError;

#[derive(Serialize)]
pub struct PickerResponse {
    pub path: Option<String>,
    pub paths: Vec<String>,
}

fn run_powershell_utf8(script: &str) -> Result<String, KanbanError> {
    let wrapped = format!(
        "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; $OutputEncoding = [System.Text.Encoding]::UTF8; {}",
        script
    );
    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-Command", &wrapped])
        .output()
        .map_err(|e| KanbanError::Internal(format!("Failed to launch picker: {}", e)))?;

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub async fn pick_directory() -> Result<Json<PickerResponse>, KanbanError> {
    let raw = run_powershell_utf8(
        r#"Add-Type -AssemblyName System.Windows.Forms; $f = New-Object System.Windows.Forms.FolderBrowserDialog; $f.Description = 'Select Working Directory'; if ($f.ShowDialog() -eq 'OK') { $f.SelectedPath } else { '' }"#,
    )?;

    let path = if raw.is_empty() { None } else { Some(raw) };

    Ok(Json(PickerResponse {
        path,
        paths: vec![],
    }))
}

pub async fn pick_files() -> Result<Json<PickerResponse>, KanbanError> {
    let raw = run_powershell_utf8(
        r#"Add-Type -AssemblyName System.Windows.Forms; $f = New-Object System.Windows.Forms.OpenFileDialog; $f.Multiselect = $true; $f.Title = 'Select Files to Link'; if ($f.ShowDialog() -eq 'OK') { $f.FileNames -join '|' } else { '' }"#,
    )?;

    let paths: Vec<String> = if raw.is_empty() {
        vec![]
    } else {
        raw.split('|').map(|s| s.to_string()).collect()
    };

    Ok(Json(PickerResponse {
        path: None,
        paths,
    }))
}
