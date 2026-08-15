use std::fs::{self, File};
use std::io::Write;
use std::path::{Component, Path, PathBuf};

const MAX_WORKSPACES: usize = 8;

#[derive(Clone, Debug)]
pub struct WorkspaceRegistry {
    roots: Vec<PathBuf>,
}

#[derive(Clone, Debug)]
pub struct ResolvedWorkspacePath {
    pub root: PathBuf,
    pub path: PathBuf,
    pub relative_path: PathBuf,
}

impl WorkspaceRegistry {
    pub fn from_settings(values: &[String]) -> Result<Self, String> {
        if values.len() > MAX_WORKSPACES {
            return Err(format!(
                "Developer mode supports up to {MAX_WORKSPACES} workspaces."
            ));
        }

        let mut roots = Vec::new();

        for value in values {
            let value = clean_path_input(value);

            if value.is_empty() {
                continue;
            }

            let root = fs::canonicalize(&value)
                .map_err(|error| format!("Failed to read workspace `{value}`: {error}"))?;

            if !root.is_dir() {
                return Err(format!("Workspace `{value}` is not a folder."));
            }

            if is_protected_path(&root) {
                return Err("A protected folder cannot be used as a Waey workspace.".to_string());
            }

            if !roots.iter().any(|existing| existing == &root) {
                roots.push(root);
            }
        }

        Ok(Self { roots })
    }

    pub fn roots(&self) -> &[PathBuf] {
        &self.roots
    }

    pub fn is_empty(&self) -> bool {
        self.roots.is_empty()
    }

    pub fn root_for_path(&self, path: &Path) -> Option<&Path> {
        self.roots
            .iter()
            .find(|root| path.starts_with(root))
            .map(PathBuf::as_path)
    }

    pub fn resolve_existing(
        &self,
        workspace: Option<&str>,
        relative_path: &str,
    ) -> Result<ResolvedWorkspacePath, String> {
        let relative_path = validated_relative_path(relative_path)?;
        let roots = self.candidate_roots(workspace)?;
        let mut matches = Vec::new();

        for root in roots {
            let candidate = root.join(&relative_path);

            if !candidate.is_file() {
                continue;
            }

            let canonical_path = fs::canonicalize(&candidate)
                .map_err(|error| format!("Failed to resolve workspace file: {error}"))?;

            if !canonical_path.starts_with(&root) {
                continue;
            }

            matches.push(ResolvedWorkspacePath {
                root,
                path: canonical_path,
                relative_path: relative_path.clone(),
            });
        }

        match matches.len() {
            0 => Err("The requested file was not found inside an allowed workspace.".to_string()),
            1 => Ok(matches.remove(0)),
            _ => Err("This relative path is ambiguous across multiple workspaces. Include the workspace field.".to_string()),
        }
    }

    pub fn resolve_write_target(
        &self,
        workspace: Option<&str>,
        relative_path: &str,
        allow_overwrite: bool,
    ) -> Result<ResolvedWorkspacePath, String> {
        let relative_path = validated_relative_path(relative_path)?;
        let roots = self.candidate_roots(workspace)?;
        let mut matches = Vec::new();

        for root in roots {
            let candidate = root.join(&relative_path);

            if candidate.exists() {
                let canonical_path = fs::canonicalize(&candidate)
                    .map_err(|error| format!("Failed to resolve target file: {error}"))?;

                if !canonical_path.is_file() || !canonical_path.starts_with(&root) {
                    continue;
                }

                if !allow_overwrite {
                    return Err(
                        "The target file already exists. Use an edit action for existing files."
                            .to_string(),
                    );
                }

                matches.push(ResolvedWorkspacePath {
                    root,
                    path: canonical_path,
                    relative_path: relative_path.clone(),
                });
                continue;
            }

            let parent = candidate.parent().ok_or_else(|| {
                "New files must have a parent folder inside an allowed workspace.".to_string()
            })?;
            let canonical_parent = fs::canonicalize(parent)
                .map_err(|error| format!("Failed to resolve target folder: {error}"))?;

            if !canonical_parent.is_dir() || !canonical_parent.starts_with(&root) {
                continue;
            }

            matches.push(ResolvedWorkspacePath {
                root,
                path: canonical_parent.join(
                    relative_path
                        .file_name()
                        .ok_or_else(|| "New file path is missing a file name.".to_string())?,
                ),
                relative_path: relative_path.clone(),
            });
        }

        match matches.len() {
            0 => Err("The target folder is not inside an allowed workspace.".to_string()),
            1 => Ok(matches.remove(0)),
            _ => Err("This relative path is ambiguous across multiple workspaces. Include the workspace field.".to_string()),
        }
    }

    fn candidate_roots(&self, workspace: Option<&str>) -> Result<Vec<PathBuf>, String> {
        let Some(workspace) = workspace
            .map(clean_path_input)
            .filter(|value| !value.is_empty())
        else {
            return Ok(self.roots.clone());
        };

        let selected_root = fs::canonicalize(&workspace)
            .map_err(|_| "The requested workspace is not available.".to_string())?;

        self.roots
            .iter()
            .find(|root| *root == &selected_root)
            .cloned()
            .map(|root| vec![root])
            .ok_or_else(|| "The requested workspace is not approved for Waey.".to_string())
    }
}

pub fn atomic_write_text(path: &Path, content: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "Target file has no parent folder.".to_string())?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "Target file has no valid name.".to_string())?;
    let token = uuid::Uuid::new_v4();
    let temporary_path = parent.join(format!(".{file_name}.{token}.waey-tmp"));
    let write_result = (|| -> Result<(), String> {
        let mut file = File::create(&temporary_path).map_err(|error| error.to_string())?;
        file.write_all(content.as_bytes())
            .map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        replace_file_transactionally(&temporary_path, path)
    })();

    let _ = fs::remove_file(&temporary_path);

    write_result
}

pub fn replace_file_transactionally(temporary_path: &Path, path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "Target file has no parent folder.".to_string())?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "Target file has no valid name.".to_string())?;
    let backup_path = parent.join(format!(".{file_name}.{}.waey-backup", uuid::Uuid::new_v4()));

    if !path.exists() {
        return fs::rename(temporary_path, path).map_err(|error| error.to_string());
    }

    fs::rename(path, &backup_path).map_err(|error| error.to_string())?;

    if let Err(error) = fs::rename(temporary_path, path) {
        let _ = fs::rename(&backup_path, path);
        return Err(error.to_string());
    }

    let _ = fs::remove_file(&backup_path);
    Ok(())
}

pub fn is_protected_path(path: &Path) -> bool {
    path.components().any(|component| match component {
        Component::Normal(value) => {
            let value = value.to_string_lossy().to_ascii_lowercase();

            value == ".git"
                || value == ".env"
                || value.starts_with(".env.")
                || value.ends_with(".pem")
                || value.ends_with(".key")
                || value.ends_with(".pfx")
                || value.ends_with(".p12")
                || value == "id_rsa"
                || value == "id_ed25519"
                || value.contains("secret")
                || value.contains("credential")
        }
        _ => false,
    })
}

pub fn should_skip_workspace_path(path: &Path) -> bool {
    path.components().any(|component| match component {
        Component::Normal(value) => matches!(
            value.to_string_lossy().to_ascii_lowercase().as_str(),
            ".git"
                | "node_modules"
                | "dist"
                | "build"
                | "target"
                | ".next"
                | ".turbo"
                | "coverage"
                | ".venv"
                | "__pycache__"
        ),
        _ => false,
    })
}

pub fn looks_like_text_file(path: &Path) -> bool {
    let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
        return false;
    };

    matches!(
        extension.to_ascii_lowercase().as_str(),
        "rs" | "ts"
            | "tsx"
            | "js"
            | "jsx"
            | "json"
            | "css"
            | "html"
            | "md"
            | "py"
            | "go"
            | "java"
            | "cs"
            | "cpp"
            | "c"
            | "h"
            | "hpp"
            | "toml"
            | "yaml"
            | "yml"
            | "sql"
            | "vue"
            | "svelte"
            | "astro"
            | "php"
            | "rb"
            | "swift"
            | "kt"
            | "kts"
            | "dart"
            | "sh"
            | "ps1"
            | "lua"
            | "r"
            | "ex"
            | "exs"
            | "scala"
            | "pl"
            | "pm"
            | "fs"
            | "fsx"
            | "clj"
            | "cljs"
            | "erl"
            | "hrl"
            | "zig"
            | "dockerfile"
    )
}

pub fn clean_path_input(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim()
        .to_string()
}

fn validated_relative_path(value: &str) -> Result<PathBuf, String> {
    let value = clean_path_input(value);

    if value.is_empty() {
        return Err("A workspace-relative file path is required.".to_string());
    }

    let path = PathBuf::from(&value);

    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err("File paths must be relative to an approved workspace.".to_string());
    }

    if is_protected_path(&path) {
        return Err("Waey will not access protected or secret-like files.".to_string());
    }

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::{is_protected_path, looks_like_text_file, validated_relative_path};
    use std::path::Path;

    #[test]
    fn rejects_path_traversal() {
        assert!(validated_relative_path("../outside.rs").is_err());
        assert!(validated_relative_path("C:\\outside.rs").is_err());
    }

    #[test]
    fn blocks_secret_like_files() {
        assert!(is_protected_path(Path::new("src/.env.local")));
        assert!(is_protected_path(Path::new("keys/id_ed25519")));
    }

    #[test]
    fn recognizes_supported_text_files() {
        assert!(looks_like_text_file(Path::new("src/App.tsx")));
        assert!(!looks_like_text_file(Path::new("sheet.xlsx")));
    }
}
