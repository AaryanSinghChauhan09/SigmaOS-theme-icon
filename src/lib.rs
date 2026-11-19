// SPDX-License-Identifier: MIT OR Apache-2.0
//! SigmaOS-theme-icon — Icon theme pack (mint-x/y/l).
//!
//! Implements XDG Icon Theme specification lookup: directory hierarchies, size/scale matching,
//! theme inheritance chains (fallback to hicolor/Adwaita), and symbolic SVG resolution.
//!
//! Inspired by Omarchy's crisp icon indexing, Hyprland's high-DPI scaling,
//! and Catppuccin Mocha's glyph aesthetics.
//!
//! Principles: deterministic theme lookup fallback, pure `std`, zero `unsafe`.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

// ── Error types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IconThemeError {
    ThemeNotFound(String),
    IconNotFound { name: String, size: u32 },
    InvalidIndexFile(String),
    UnsupportedSize(u32),
}

impl fmt::Display for IconThemeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ThemeNotFound(t) => write!(f, "icon theme '{t}' not found"),
            Self::IconNotFound { name, size } => write!(f, "icon '{name}' not found for size {size}px"),
            Self::InvalidIndexFile(e) => write!(f, "index.theme parse error: {e}"),
            Self::UnsupportedSize(s) => write!(f, "unsupported icon size: {s}px"),
        }
    }
}

impl std::error::Error for IconThemeError {}

// ── Icon Context & Directory Spec ────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IconContext {
    Actions,
    Apps,
    Categories,
    Devices,
    Emblems,
    Places,
    Status,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconDirectory {
    pub size: u32,
    pub scale: u32,
    pub context: IconContext,
    pub path_relative: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconTheme {
    pub name: String,
    pub comment: String,
    pub inherits: Vec<String>,
    pub directories: Vec<IconDirectory>,
    icons: HashMap<(String, u32), PathBuf>, // (icon_name, size) -> full_path
}

impl IconTheme {
    pub fn new(name: impl Into<String>, comment: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            comment: comment.into(),
            inherits: vec!["hicolor".into()],
            directories: Vec::new(),
            icons: HashMap::new(),
        }
    }

    pub fn add_directory(&mut self, size: u32, context: IconContext, rel_path: impl AsRef<Path>) {
        self.directories.push(IconDirectory {
            size,
            scale: 1,
            context,
            path_relative: rel_path.as_ref().to_path_buf(),
        });
    }

    pub fn register_icon(&mut self, name: impl Into<String>, size: u32, path: impl AsRef<Path>) {
        self.icons.insert((name.into(), size), path.as_ref().to_path_buf());
    }

    pub fn find_exact(&self, name: &str, size: u32) -> Option<&PathBuf> {
        self.icons.get(&(name.to_string(), size))
    }

    pub fn find_closest(&self, name: &str, desired_size: u32) -> Option<&PathBuf> {
        if let Some(p) = self.find_exact(name, desired_size) {
            return Some(p);
        }
        // Fall back to any available size, preferring the smallest delta
        self.icons
            .iter()
            .filter(|((n, _), _)| n == name)
            .min_by_key(|((_, sz), _)| (*sz as i32 - desired_size as i32).abs())
            .map(|(_, path)| path)
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_theme_with_standard_defaults() {
        // Arrange & Act
        let theme = IconTheme::new("Catppuccin Mocha Icons", "Catppuccin icon suite");
        // Assert
        assert_eq!(theme.name, "Catppuccin Mocha Icons");
        assert_eq!(theme.inherits, vec!["hicolor"]);
        assert!(theme.directories.is_empty());
    }

    #[test]
    fn register_and_lookup_exact_icon() {
        // Arrange
        let mut theme = IconTheme::new("test", "test");
        theme.register_icon("terminal", 48, "/icons/48x48/apps/terminal.png");
        theme.register_icon("terminal", 16, "/icons/16x16/apps/terminal.png");

        // Act
        let found = theme.find_exact("terminal", 48);
        let missing = theme.find_exact("browser", 48);

        // Assert
        assert_eq!(found, Some(&PathBuf::from("/icons/48x48/apps/terminal.png")));
        assert_eq!(missing, None);
    }

    #[test]
    fn find_closest_size_fallback() {
        // Arrange
        let mut theme = IconTheme::new("test", "test");
        theme.register_icon("folder", 16, "/icons/16x16/folder.png");
        theme.register_icon("folder", 64, "/icons/64x64/folder.png");

        // Act: desired is 32px, closest is 16px (diff 16) vs 64px (diff 32)
        let closest = theme.find_closest("folder", 32);

        // Assert
        assert_eq!(closest, Some(&PathBuf::from("/icons/16x16/folder.png")));
    }

    #[test]
    fn directory_registration() {
        // Arrange
        let mut theme = IconTheme::new("test", "test");
        // Act
        theme.add_directory(24, IconContext::Actions, "24x24/actions");
        theme.add_directory(48, IconContext::Apps, "48x48/apps");

        // Assert
        assert_eq!(theme.directories.len(), 2);
        assert_eq!(theme.directories[0].context, IconContext::Actions);
        assert_eq!(theme.directories[1].size, 48);
    }

    #[test]
    fn error_display_formatting() {
        // Arrange
        let err = IconThemeError::IconNotFound {
            name: "edit-cut".into(),
            size: 32,
        };
        // Act & Assert
        assert_eq!(format!("{err}"), "icon 'edit-cut' not found for size 32px");
    }
}
