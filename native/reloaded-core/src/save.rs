use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

use crate::error::CoreError;

/// Structured metadata extracted from `Pokemon.ini`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct PokemonIniData {
    pub trainer_name: Option<String>,
    pub badges: Option<u32>,
    pub pokedex_count: Option<u32>,
    pub play_time: Option<String>,
    pub money: Option<u64>,
    pub difficulty: Option<String>,
    pub raw_entries: HashMap<String, String>,
}

/// Metadata and status for a game save slot (`Save1.sav`, `Save2.sav`, `Save3.sav`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SaveSlotInfo {
    pub slot_number: u32,
    pub filename: String,
    pub file_path: String,
    pub exists: bool,
    pub size_bytes: u64,
    pub modified_timestamp: u64,
    pub ini_data: Option<PokemonIniData>,
}

/// Manager for inspecting, backing up, restoring, and exporting save games.
#[derive(Debug, Clone)]
pub struct SaveManager {
    base_dir: PathBuf,
    backup_dir: PathBuf,
}

impl SaveManager {
    /// Creates a new `SaveManager` pointing to the game storage directory.
    pub fn new<P: Into<PathBuf>>(base_dir: P) -> Self {
        let base = base_dir.into();
        let backup = base.join("backups");
        Self {
            base_dir: base,
            backup_dir: backup,
        }
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    pub fn backup_dir(&self) -> &Path {
        &self.backup_dir
    }

    /// Ensures the base directory and `backups/` subfolder exist.
    pub fn ensure_directories(&self) -> Result<(), CoreError> {
        fs::create_dir_all(&self.base_dir)?;
        fs::create_dir_all(&self.backup_dir)?;
        Ok(())
    }

    /// Returns the path to the save file for a given slot (e.g. 1 -> Save1.sav).
    pub fn slot_path(&self, slot: u32) -> PathBuf {
        self.base_dir.join(format!("Save{}.sav", slot))
    }

    /// Returns the path to `Pokemon.ini`.
    pub fn ini_path(&self) -> PathBuf {
        self.base_dir.join("Pokemon.ini")
    }

    /// Parses an INI string into `PokemonIniData`.
    pub fn parse_ini_string(content: &str) -> PokemonIniData {
        let mut data = PokemonIniData::default();
        let mut current_section = String::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#') {
                continue;
            }

            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                current_section = trimmed[1..trimmed.len() - 1].trim().to_string();
                continue;
            }

            if let Some((raw_key, raw_val)) = trimmed.split_once('=') {
                let key = raw_key.trim();
                let mut val = raw_val.trim();

                // Strip quotes if present
                if val.starts_with('"') && val.ends_with('"') && val.len() >= 2 {
                    val = &val[1..val.len() - 1];
                }

                let qualified_key = if current_section.is_empty() {
                    key.to_string()
                } else {
                    format!("{}.{}", current_section, key)
                };

                data.raw_entries
                    .insert(qualified_key.clone(), val.to_string());
                data.raw_entries.insert(key.to_string(), val.to_string());

                let lower_key = key.to_lowercase();
                if lower_key == "nombre" || lower_key == "name" || lower_key == "trainer" {
                    data.trainer_name = Some(val.to_string());
                } else if lower_key == "medallas" || lower_key == "badges" {
                    if let Ok(b) = val.parse::<u32>() {
                        data.badges = Some(b);
                    }
                } else if lower_key == "pokedex" || lower_key == "pokedex_count" || lower_key == "vistos" {
                    if let Ok(p) = val.parse::<u32>() {
                        data.pokedex_count = Some(p);
                    }
                } else if lower_key == "tiempo" || lower_key == "time" || lower_key == "playtime" {
                    data.play_time = Some(val.to_string());
                } else if lower_key == "dinero" || lower_key == "money" {
                    if let Ok(m) = val.parse::<u64>() {
                        data.money = Some(m);
                    }
                } else if lower_key == "dificultad" || lower_key == "difficulty" {
                    data.difficulty = Some(val.to_string());
                }
            }
        }

        data
    }

    /// Parses the `Pokemon.ini` file if it exists.
    pub fn parse_ini_file<P: AsRef<Path>>(path: P) -> Result<PokemonIniData, CoreError> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(CoreError::Save(format!("INI file not found: {:?}", path)));
        }

        let mut file = File::open(path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        Ok(Self::parse_ini_string(&content))
    }

    /// Inspects a specific save slot (1, 2, or 3).
    pub fn inspect_slot(&self, slot: u32) -> Result<SaveSlotInfo, CoreError> {
        let path = self.slot_path(slot);
        let filename = format!("Save{}.sav", slot);

        let (exists, size_bytes, modified_timestamp) = if path.exists() {
            let meta = fs::metadata(&path)?;
            let size = meta.len();
            let timestamp = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            (true, size, timestamp)
        } else {
            (false, 0, 0)
        };

        // Try reading Pokemon.ini for player metadata
        let ini_data = if self.ini_path().exists() {
            Self::parse_ini_file(self.ini_path()).ok()
        } else {
            None
        };

        Ok(SaveSlotInfo {
            slot_number: slot,
            filename,
            file_path: path.to_string_lossy().to_string(),
            exists,
            size_bytes,
            modified_timestamp,
            ini_data,
        })
    }

    /// Lists status for all 3 standard GameMaker save slots (Save1.sav, Save2.sav, Save3.sav).
    pub fn list_save_slots(&self) -> Result<Vec<SaveSlotInfo>, CoreError> {
        let mut slots = Vec::with_capacity(3);
        for slot in 1..=3 {
            slots.push(self.inspect_slot(slot)?);
        }
        Ok(slots)
    }

    /// Creates a timestamped backup of the specified save slot into `backups/`.
    pub fn backup_save(&self, slot: u32) -> Result<PathBuf, CoreError> {
        self.ensure_directories()?;
        let source_path = self.slot_path(slot);

        if !source_path.exists() {
            return Err(CoreError::Save(format!(
                "Cannot backup: save slot {} does not exist ({:?})",
                slot, source_path
            )));
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let backup_filename = format!("Save{}_{}.sav", slot, timestamp);
        let backup_path = self.backup_dir.join(backup_filename);

        fs::copy(&source_path, &backup_path)?;

        // If Pokemon.ini exists, back it up alongside the save
        let ini_src = self.ini_path();
        if ini_src.exists() {
            let ini_backup_filename = format!("Pokemon_{}_{}.ini", slot, timestamp);
            let ini_backup_path = self.backup_dir.join(ini_backup_filename);
            let _ = fs::copy(&ini_src, ini_backup_path);
        }

        Ok(backup_path)
    }

    /// Lists all backup save files in `backups/`.
    pub fn list_backups(&self, slot: Option<u32>) -> Result<Vec<PathBuf>, CoreError> {
        if !self.backup_dir.exists() {
            return Ok(Vec::new());
        }

        let mut backups = Vec::new();
        for entry in fs::read_dir(&self.backup_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.ends_with(".sav") {
                        if let Some(target_slot) = slot {
                            let prefix = format!("Save{}_", target_slot);
                            if name.starts_with(&prefix) {
                                backups.push(path);
                            }
                        } else {
                            backups.push(path);
                        }
                    }
                }
            }
        }

        backups.sort();
        Ok(backups)
    }

    /// Restores a backup file over a specified save slot.
    pub fn restore_backup(&self, backup_file: &Path, slot: u32) -> Result<(), CoreError> {
        if !backup_file.exists() {
            return Err(CoreError::Save(format!(
                "Backup file does not exist: {:?}",
                backup_file
            )));
        }

        self.ensure_directories()?;
        let target_path = self.slot_path(slot);
        fs::copy(backup_file, &target_path)?;

        // Also check if there is an associated ini file backup
        if let Some(stem) = backup_file.file_stem().and_then(|s| s.to_str()) {
            if let Some(timestamp) = stem.split('_').last() {
                let candidate_ini = self.backup_dir.join(format!("Pokemon_{}_{}.ini", slot, timestamp));
                if candidate_ini.exists() {
                    let _ = fs::copy(&candidate_ini, self.ini_path());
                }
            }
        }

        Ok(())
    }

    /// Exports a save slot to an external destination path.
    pub fn export_save(&self, slot: u32, target_path: &Path) -> Result<(), CoreError> {
        let source_path = self.slot_path(slot);
        if !source_path.exists() {
            return Err(CoreError::Save(format!(
                "Save slot {} does not exist",
                slot
            )));
        }

        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&source_path, target_path)?;
        Ok(())
    }

    /// Imports an external save file into a given slot.
    pub fn import_save(&self, source_path: &Path, slot: u32) -> Result<(), CoreError> {
        if !source_path.exists() {
            return Err(CoreError::Save(format!(
                "Import source does not exist: {:?}",
                source_path
            )));
        }

        self.ensure_directories()?;
        let target_path = self.slot_path(slot);
        fs::copy(source_path, &target_path)?;
        Ok(())
    }

    /// Deletes a save slot.
    pub fn delete_save(&self, slot: u32) -> Result<(), CoreError> {
        let path = self.slot_path(slot);
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_pokemon_ini_parsing() {
        let sample_ini = r#"
        ; Pokemon Reloaded Configuration
        [Partida]
        Nombre = Ash Ketchum
        Medallas = 16
        Pokedex = 493
        Tiempo = 142:15:30
        Dinero = 999999
        Dificultad = Experto
        "#;

        let ini_data = SaveManager::parse_ini_string(sample_ini);
        assert_eq!(ini_data.trainer_name.as_deref(), Some("Ash Ketchum"));
        assert_eq!(ini_data.badges, Some(16));
        assert_eq!(ini_data.pokedex_count, Some(493));
        assert_eq!(ini_data.play_time.as_deref(), Some("142:15:30"));
        assert_eq!(ini_data.money, Some(999999));
        assert_eq!(ini_data.difficulty.as_deref(), Some("Experto"));
    }

    #[test]
    fn test_save_manager_backup_and_restore() {
        let test_dir = std::env::temp_dir().join(format!("reloaded_save_test_{}", std::process::id()));
        let save_mgr = SaveManager::new(&test_dir);
        save_mgr.ensure_directories().expect("Create directories");

        // Create Save1.sav and Pokemon.ini
        let save1_file = save_mgr.slot_path(1);
        let mut f = File::create(&save1_file).expect("Create Save1");
        f.write_all(b"SAMPLE_SAVE_GAME_DATA_BINARY").unwrap();

        let ini_file = save_mgr.ini_path();
        let mut f_ini = File::create(&ini_file).expect("Create ini");
        f_ini.write_all(b"Nombre=Red\nMedallas=8\n").unwrap();

        // Inspect slots
        let slots = save_mgr.list_save_slots().expect("List save slots");
        assert_eq!(slots.len(), 3);
        assert!(slots[0].exists);
        assert_eq!(slots[0].size_bytes, 28);
        assert!(!slots[1].exists);
        assert!(!slots[2].exists);

        let ini_data = slots[0].ini_data.as_ref().expect("Ini data present");
        assert_eq!(ini_data.trainer_name.as_deref(), Some("Red"));
        assert_eq!(ini_data.badges, Some(8));

        // Create backup
        let backup_path = save_mgr.backup_save(1).expect("Backup save 1");
        assert!(backup_path.exists());

        let backups = save_mgr.list_backups(Some(1)).expect("List backups for slot 1");
        assert_eq!(backups.len(), 1);
        assert_eq!(backups[0], backup_path);

        // Delete slot 1
        save_mgr.delete_save(1).expect("Delete save 1");
        assert!(!save_mgr.slot_path(1).exists());

        // Restore from backup
        save_mgr.restore_backup(&backup_path, 1).expect("Restore backup");
        assert!(save_mgr.slot_path(1).exists());

        let mut restored_bytes = Vec::new();
        File::open(save_mgr.slot_path(1)).unwrap().read_to_end(&mut restored_bytes).unwrap();
        assert_eq!(restored_bytes, b"SAMPLE_SAVE_GAME_DATA_BINARY");

        // Clean up
        let _ = fs::remove_dir_all(&test_dir);
    }
}
