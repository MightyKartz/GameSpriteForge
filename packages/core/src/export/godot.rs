use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::ExportError;

const GODOT_IMPORT_SCRIPT: &str = include_str!("../../../../scripts/godot/import_forge_pack.gd");

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GodotProjectExportParams {
    pub pack_dir: PathBuf,
    pub output_dir: PathBuf,
    pub project_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GodotProjectExportOutput {
    pub project_dir: PathBuf,
    pub import_script_path: PathBuf,
    pub project_file_path: PathBuf,
    pub readme_path: PathBuf,
}

pub fn export_godot_project(
    params: GodotProjectExportParams,
) -> Result<GodotProjectExportOutput, ExportError> {
    let project_slug = sanitize_project_name(&params.project_name);
    let project_dir = params.output_dir.join(project_slug);
    let addon_dir = project_dir.join("addons/game_sprite_forge");
    fs::create_dir_all(&addon_dir)?;

    let project_file_path = project_dir.join("project.godot");
    let import_script_path = addon_dir.join("import_forge_pack.gd");
    let readme_path = project_dir.join("README.md");

    fs::write(&project_file_path, project_file(&params.project_name))?;
    fs::write(&import_script_path, GODOT_IMPORT_SCRIPT)?;
    fs::write(&readme_path, readme(&params.project_name, &params.pack_dir))?;

    Ok(GodotProjectExportOutput {
        project_dir,
        import_script_path,
        project_file_path,
        readme_path,
    })
}

fn project_file(project_name: &str) -> String {
    format!(
        "; Engine configuration file.\n\nconfig_version=5\n\n[application]\n\nconfig/name=\"{}\"\nconfig/features=PackedStringArray(\"4.6\")\n",
        project_name.replace('"', "'")
    )
}

fn readme(project_name: &str, pack_dir: &Path) -> String {
    format!(
        "# {project_name}\n\nRun this command from the project folder:\n\n```bash\n/Applications/Godot.app/Contents/MacOS/Godot --headless --path . --script addons/game_sprite_forge/import_forge_pack.gd -- {}\n```\n",
        pack_dir.display()
    )
}

fn sanitize_project_name(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = sanitized.trim_matches('-');
    if trimmed.is_empty() {
        "Godot-Export".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn godot_project_export_writes_project_script_and_readme() {
        let temp = tempfile::tempdir().unwrap();
        let pack_dir = temp.path().join("Godot-Smoke-Walk.gsfpack");
        fs::create_dir_all(pack_dir.join("assets")).unwrap();
        fs::write(
            pack_dir.join("forgepack.json"),
            r#"{"name":"Godot Smoke Walk"}"#,
        )
        .unwrap();

        let output = export_godot_project(GodotProjectExportParams {
            pack_dir: pack_dir.clone(),
            output_dir: temp.path().join("godot-export"),
            project_name: "Godot Smoke Walk".to_string(),
        })
        .unwrap();

        assert!(output.project_dir.join("project.godot").is_file());
        assert!(output
            .project_dir
            .join("addons/game_sprite_forge/import_forge_pack.gd")
            .is_file());
        assert!(output.project_dir.join("README.md").is_file());

        let readme = fs::read_to_string(output.project_dir.join("README.md")).unwrap();
        assert!(readme.contains("Godot Smoke Walk"));
        assert!(readme.contains("Godot-Smoke-Walk.gsfpack"));
    }
}
