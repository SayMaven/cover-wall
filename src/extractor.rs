//! Pure Rust FLAC album cover extractor using `lofty`.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use lofty::file::TaggedFileExt;
use lofty::picture::{MimeType, PictureType};
use lofty::probe::Probe;
use lofty::tag::Accessor;
use thiserror::Error;
use walkdir::WalkDir;

const FOLDER_COVER_STEMS: [&str; 3] = ["cover", "folder", "front"];
const IMAGE_EXTENSIONS: [&str; 5] = ["jpg", "jpeg", "png", "webp", "bmp"];

/// Errors produced by the cover extractor.
#[derive(Debug, Error)]
pub enum ExtractError {
    #[error("Source directory does not exist: {0}")]
    SourceNotFound(PathBuf),
    #[error("Failed to create output directory: {0}")]
    OutputCreateFailed(#[from] std::io::Error),
}

/// Input/output configuration for an extraction run.
#[derive(Debug, Clone)]
pub struct ExtractConfig {
    pub source_dir: PathBuf,
    pub output_dir: PathBuf,
}

/// Where an extracted cover came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoverOrigin {
    Embedded,
    Folder,
    Missing,
}

/// Single album extraction result.
#[derive(Debug, Clone)]
pub struct ExtractItem {
    pub album: String,
    pub file_name: Option<String>,
    pub origin: CoverOrigin,
    pub detail: String,
}

/// Progress update sent from the background worker.
#[derive(Debug, Clone)]
pub struct ExtractProgress {
    pub processed: usize,
    pub total: usize,
    pub current_album: String,
}

/// Final summary of an extraction run.
#[derive(Debug, Clone)]
pub struct ExtractSummary {
    pub total_flacs: usize,
    pub extracted_count: usize,
    pub missing_count: usize,
    pub items: Vec<ExtractItem>,
    pub cancelled: bool,
}

/// Sanitize a string so it is safe as a Windows filename.
pub fn sanitize_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if (c as u32) < 0x20 => '_',
            c => c,
        })
        .collect();
    let trimmed = cleaned.trim().trim_end_matches('.');
    if trimmed.is_empty() {
        "Unknown Album".to_owned()
    } else {
        trimmed.to_owned()
    }
}

fn is_disc_folder(name: &str) -> bool {
    let lower = name.trim().to_ascii_lowercase();
    let rest = if let Some(r) = lower.strip_prefix("cd") {
        r.trim()
    } else if let Some(r) = lower.strip_prefix("disc") {
        r.trim()
    } else {
        return false;
    };
    !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit())
}

/// Determine album name from tag or fallback to parent folder (handling `CD n` / `Disc n`).
pub fn resolve_album_name(tag_album: Option<&str>, flac_path: &Path, root_dir: &Path) -> String {
    if let Some(album) = tag_album
        && !album.trim().is_empty()
    {
        return album.trim().to_owned();
    }

    if let Ok(rel) = flac_path.strip_prefix(root_dir)
        && rel.components().count() > 1
        && let Some(parent) = flac_path.parent()
        && let Some(parent_name) = parent.file_name().and_then(|n| n.to_str())
    {
        if is_disc_folder(parent_name)
            && let Some(grandparent) = parent.parent()
            && grandparent != root_dir
            && let Some(gp_name) = grandparent.file_name().and_then(|n| n.to_str())
        {
            return gp_name.to_owned();
        }
        return parent_name.to_owned();
    }

    flac_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown Album")
        .to_owned()
}

fn extension_from_mime(mime: Option<&MimeType>) -> &'static str {
    match mime {
        Some(MimeType::Png) => ".png",
        Some(MimeType::Bmp) => ".bmp",
        Some(MimeType::Gif) => ".gif",
        Some(MimeType::Tiff) => ".tiff",
        _ => ".jpg",
    }
}

fn find_folder_cover(flac_path: &Path, root_dir: &Path) -> Option<PathBuf> {
    let mut search_dirs = Vec::new();
    if let Some(parent) = flac_path.parent() {
        search_dirs.push(parent.to_path_buf());
        if parent != root_dir
            && let Some(grandparent) = parent.parent()
            && grandparent != root_dir
        {
            search_dirs.push(grandparent.to_path_buf());
        }
    }

    for folder in search_dirs {
        let Ok(entries) = fs::read_dir(&folder) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let stem_match = path
                .file_stem()
                .and_then(|s| s.to_str())
                .is_some_and(|s| FOLDER_COVER_STEMS.contains(&s.to_ascii_lowercase().as_str()));
            let ext_match = path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| IMAGE_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()));
            if stem_match && ext_match {
                return Some(path);
            }
        }
    }
    None
}

/// Extract unique album covers from all `.flac` files under `config.source_dir`.
pub fn run_extraction(
    config: &ExtractConfig,
    cancel: &AtomicBool,
    mut on_progress: impl FnMut(ExtractProgress),
) -> Result<ExtractSummary, ExtractError> {
    if !config.source_dir.exists() {
        return Err(ExtractError::SourceNotFound(config.source_dir.clone()));
    }
    fs::create_dir_all(&config.output_dir)?;

    let mut flac_files: Vec<PathBuf> = WalkDir::new(&config.source_dir)
        .into_iter()
        .filter_map(Result::ok)
        .map(|e| e.into_path())
        .filter(|p| {
            p.is_file()
                && p.extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| e.eq_ignore_ascii_case("flac"))
        })
        .collect();
    flac_files.sort();

    let total = flac_files.len();
    let mut seen_albums: HashSet<String> = HashSet::new();
    let mut items = Vec::new();
    let mut extracted_count = 0_usize;
    let mut missing_count = 0_usize;
    let mut cancelled = false;

    for (idx, flac_path) in flac_files.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            cancelled = true;
            break;
        }

        let tagged_file = Probe::open(flac_path).ok().and_then(|p| p.read().ok());
        let primary_tag = tagged_file
            .as_ref()
            .and_then(|f| f.primary_tag().or_else(|| f.first_tag()));

        let tag_album = primary_tag.and_then(|t| t.album());
        let album_name = resolve_album_name(tag_album.as_deref(), flac_path, &config.source_dir);
        let safe_name = sanitize_filename(&album_name);
        let key = safe_name.to_ascii_lowercase();

        on_progress(ExtractProgress {
            processed: idx + 1,
            total,
            current_album: album_name.clone(),
        });

        if !seen_albums.insert(key) {
            continue;
        }

        if let Some(tag) = primary_tag {
            let pictures = tag.pictures();
            let chosen = pictures
                .iter()
                .find(|p| p.pic_type() == PictureType::CoverFront)
                .or_else(|| pictures.first());

            if let Some(pic) = chosen
                && !pic.data().is_empty()
            {
                let ext = extension_from_mime(pic.mime_type());
                let file_name = format!("{safe_name}{ext}");
                let out_path = config.output_dir.join(&file_name);
                if fs::write(&out_path, pic.data()).is_ok() {
                    extracted_count += 1;
                    items.push(ExtractItem {
                        album: album_name,
                        file_name: Some(file_name),
                        origin: CoverOrigin::Embedded,
                        detail: "Embedded FLAC picture".to_owned(),
                    });
                    continue;
                }
            }
        }

        if let Some(folder_cover) = find_folder_cover(flac_path, &config.source_dir) {
            let ext = folder_cover
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| format!(".{}", e.to_ascii_lowercase()))
                .unwrap_or_else(|| ".jpg".to_owned());
            let file_name = format!("{safe_name}{ext}");
            let out_path = config.output_dir.join(&file_name);
            if fs::copy(&folder_cover, &out_path).is_ok() {
                extracted_count += 1;
                items.push(ExtractItem {
                    album: album_name,
                    file_name: Some(file_name),
                    origin: CoverOrigin::Folder,
                    detail: folder_cover
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("folder image")
                        .to_owned(),
                });
                continue;
            }
        }

        missing_count += 1;
        items.push(ExtractItem {
            album: album_name,
            file_name: None,
            origin: CoverOrigin::Missing,
            detail: flac_path.display().to_string(),
        });
    }

    Ok(ExtractSummary {
        total_flacs: total,
        extracted_count,
        missing_count,
        items,
        cancelled,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_filename_replaces_invalid_windows_chars() {
        assert_eq!(sanitize_filename("Best: 2009/2015*?."), "Best_ 2009_2015__");
        assert_eq!(sanitize_filename("   ... "), "Unknown Album");
    }

    #[test]
    fn resolve_album_name_handles_disc_subfolder_fallback() {
        let root = Path::new("E:/Music");
        let flac = Path::new("E:/Music/My Album/CD 1/01.flac");
        assert_eq!(resolve_album_name(None, flac, root), "My Album");
    }
}
