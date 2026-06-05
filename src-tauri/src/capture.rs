use screenshots::Screen;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_RETAINED_CAPTURES: usize = 10;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenCapture {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub origin_x: i32,
    pub origin_y: i32,
    pub source: CaptureSource,
    pub created_at: u128,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CaptureSource {
    FullScreen,
    Region,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

pub fn capture_full_screen() -> Result<ScreenCapture, String> {
    let screen = Screen::all()
        .map_err(|error| error.to_string())?
        .into_iter()
        .next()
        .ok_or_else(|| "No screen was available for capture.".to_string())?;

    let image = screen.capture().map_err(|error| error.to_string())?;
    let created_at = timestamp_millis()?;
    let path = capture_path("full", created_at)?;

    image.save(&path).map_err(|error| error.to_string())?;
    prune_old_captures();

    Ok(ScreenCapture {
        path: path_to_string(path),
        width: image.width(),
        height: image.height(),
        origin_x: screen.display_info.x,
        origin_y: screen.display_info.y,
        source: CaptureSource::FullScreen,
        created_at,
    })
}

pub fn capture_screen_region(rect: CaptureRect) -> Result<ScreenCapture, String> {
    if rect.width == 0 || rect.height == 0 {
        return Err("Capture region must have a positive width and height.".to_string());
    }

    let screen = Screen::from_point(rect.x, rect.y).map_err(|error| error.to_string())?;
    let image = screen
        .capture_area(rect.x, rect.y, rect.width, rect.height)
        .map_err(|error| error.to_string())?;
    let created_at = timestamp_millis()?;
    let path = capture_path("region", created_at)?;

    image.save(&path).map_err(|error| error.to_string())?;
    prune_old_captures();

    Ok(ScreenCapture {
        path: path_to_string(path),
        width: image.width(),
        height: image.height(),
        origin_x: rect.x,
        origin_y: rect.y,
        source: CaptureSource::Region,
        created_at,
    })
}

fn capture_path(prefix: &str, created_at: u128) -> Result<PathBuf, String> {
    let directory = std::env::temp_dir().join("waey").join("captures");
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;

    Ok(directory.join(format!("{prefix}-{created_at}.png")))
}

fn prune_old_captures() {
    let directory = std::env::temp_dir().join("waey").join("captures");

    let Ok(entries) = fs::read_dir(&directory) else {
        return;
    };

    let mut files: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("png"))
        .collect();

    if files.len() <= MAX_RETAINED_CAPTURES {
        return;
    }

    files.sort_by_key(|path| {
        path.metadata()
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH)
    });

    for path in files.iter().take(files.len() - MAX_RETAINED_CAPTURES) {
        let _ = fs::remove_file(path);
    }
}

fn timestamp_millis() -> Result<u128, String> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_millis())
}

fn path_to_string(path: PathBuf) -> String {
    path.to_string_lossy().to_string()
}
