use std::env;
use std::path::{Path, PathBuf};

use super::VideoError;

pub const FFMPEG_MISSING_CODE: &str = "ffmpeg_missing";
pub const FFMPEG_MISSING_MESSAGE: &str = "Install ffmpeg or choose an ffmpeg binary in Settings.";

#[derive(Debug, Clone, Default)]
pub struct FfmpegSearch {
    pub configured_ffmpeg_path: Option<PathBuf>,
    pub configured_ffprobe_path: Option<PathBuf>,
    pub bundled_resource_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfmpegPaths {
    pub ffmpeg_path: PathBuf,
    pub ffprobe_path: PathBuf,
}

pub fn resolve_ffmpeg_paths(search: &FfmpegSearch) -> Result<FfmpegPaths, VideoError> {
    let ffmpeg_path = resolve_binary(
        "ffmpeg",
        search.configured_ffmpeg_path.as_deref(),
        search.bundled_resource_path.as_deref(),
    )?;
    let ffprobe_path = resolve_binary(
        "ffprobe",
        search.configured_ffprobe_path.as_deref(),
        search.bundled_resource_path.as_deref(),
    )?;

    Ok(FfmpegPaths {
        ffmpeg_path,
        ffprobe_path,
    })
}

pub fn resolve_binary(
    binary_name: &str,
    configured_path: Option<&Path>,
    bundled_resource_path: Option<&Path>,
) -> Result<PathBuf, VideoError> {
    if let Some(path) = configured_path.and_then(existing_file) {
        return Ok(path);
    }

    if let Some(path) = bundled_resource_path.and_then(|resource_path| {
        bundled_candidates(binary_name, resource_path)
            .into_iter()
            .find_map(|candidate| existing_file(candidate.as_path()))
    }) {
        return Ok(path);
    }

    find_in_path(binary_name)
        .map(PathBuf::from)
        .ok_or_else(VideoError::ffmpeg_missing)
}

pub fn find_in_path(binary_name: &str) -> Option<String> {
    find_in_directories(binary_name, tool_search_directories())
}

fn find_in_directories<I>(binary_name: &str, directories: I) -> Option<String>
where
    I: IntoIterator<Item = PathBuf>,
{
    directories
        .into_iter()
        .flat_map(|directory| path_candidates(binary_name, &directory))
        .find_map(|candidate| existing_file(candidate.as_path()))
        .map(|path| path.to_string_lossy().into_owned())
}

fn tool_search_directories() -> Vec<PathBuf> {
    let mut directories = env::var_os("GAME_SPRITE_FORGE_FFMPEG_SEARCH_DIRS")
        .map(|value| env::split_paths(&value).collect::<Vec<_>>())
        .or_else(|| env::var_os("PATH").map(|value| env::split_paths(&value).collect::<Vec<_>>()))
        .unwrap_or_default();

    if !disable_macos_default_tool_dirs() {
        for directory in default_tool_directories() {
            if !directories.iter().any(|existing| existing == &directory) {
                directories.push(directory);
            }
        }
    }

    directories
}

fn disable_macos_default_tool_dirs() -> bool {
    env_flag_enabled("GAME_SPRITE_FORGE_DISABLE_MACOS_DEFAULT_TOOL_DIRS")
}

fn env_flag_enabled(name: &str) -> bool {
    env::var_os(name)
        .and_then(|value| value.into_string().ok())
        .map(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

#[cfg(target_os = "macos")]
fn default_tool_directories() -> Vec<PathBuf> {
    vec![
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
    ]
}

#[cfg(not(target_os = "macos"))]
fn default_tool_directories() -> Vec<PathBuf> {
    Vec::new()
}

fn existing_file(path: &Path) -> Option<PathBuf> {
    if path.is_file() {
        Some(path.to_path_buf())
    } else {
        None
    }
}

fn bundled_candidates(binary_name: &str, resource_path: &Path) -> Vec<PathBuf> {
    if resource_path.is_file() {
        return if file_name_matches(binary_name, resource_path) {
            vec![resource_path.to_path_buf()]
        } else {
            Vec::new()
        };
    }

    path_candidates(binary_name, resource_path)
        .into_iter()
        .chain(path_candidates(binary_name, &resource_path.join("bin")))
        .chain(path_candidates(
            binary_name,
            &resource_path.join("Resources"),
        ))
        .chain(path_candidates(
            binary_name,
            &resource_path.join("Contents").join("Resources"),
        ))
        .collect()
}

fn path_candidates(binary_name: &str, directory: &Path) -> Vec<PathBuf> {
    let candidates = vec![directory.join(binary_name)];

    #[cfg(windows)]
    {
        if !binary_name.ends_with(".exe") {
            candidates.push(directory.join(format!("{binary_name}.exe")));
        }
    }

    candidates
}

fn file_name_matches(binary_name: &str, path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };

    let windows_binary_name = format!("{binary_name}.exe");
    file_name == binary_name || file_name == windows_binary_name.as_str()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_in_directories_uses_supplied_search_dirs() {
        let temp = tempfile::tempdir().unwrap();
        let binary = temp.path().join("ffmpeg");
        std::fs::write(&binary, b"").unwrap();

        let found = find_in_directories("ffmpeg", vec![temp.path().to_path_buf()]).unwrap();

        assert_eq!(found, binary.to_string_lossy());
    }

    #[test]
    fn env_flag_enabled_accepts_common_truthy_values() {
        for value in ["1", "true", "TRUE", "yes", "YES"] {
            temp_env::with_var(
                "GAME_SPRITE_FORGE_DISABLE_MACOS_DEFAULT_TOOL_DIRS",
                Some(value),
                || {
                    assert!(disable_macos_default_tool_dirs());
                },
            );
        }
    }

    #[test]
    fn env_search_dirs_override_path_when_set() {
        let temp = tempfile::tempdir().unwrap();
        let ffmpeg = temp.path().join("ffmpeg");
        std::fs::write(&ffmpeg, b"").unwrap();
        let search_dirs = temp.path().to_string_lossy().into_owned();

        temp_env::with_var(
            "GAME_SPRITE_FORGE_FFMPEG_SEARCH_DIRS",
            Some(search_dirs.as_str()),
            || {
                temp_env::with_var(
                    "GAME_SPRITE_FORGE_DISABLE_MACOS_DEFAULT_TOOL_DIRS",
                    Some("1"),
                    || {
                        let found = find_in_path("ffmpeg").unwrap();
                        assert_eq!(found, ffmpeg.to_string_lossy());
                    },
                );
            },
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn disable_flag_skips_macos_default_tool_dirs() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().to_string_lossy().into_owned();

        temp_env::with_var("GAME_SPRITE_FORGE_FFMPEG_SEARCH_DIRS", None::<&str>, || {
            temp_env::with_var("PATH", Some(path.as_str()), || {
                temp_env::with_var(
                    "GAME_SPRITE_FORGE_DISABLE_MACOS_DEFAULT_TOOL_DIRS",
                    Some("YES"),
                    || {
                        let dirs = tool_search_directories();

                        assert_eq!(dirs, vec![temp.path().to_path_buf()]);
                    },
                );
            });
        });
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_default_tool_dirs_cover_homebrew_locations() {
        let dirs = default_tool_directories();

        assert!(dirs.contains(&PathBuf::from("/opt/homebrew/bin")));
        assert!(dirs.contains(&PathBuf::from("/usr/local/bin")));
    }
}
