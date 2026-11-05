use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

use crate::error::CoreError;

pub const DEFAULT_MAX_ATLAS_SIZE: u32 = 2048;
pub const MANIFEST_FILENAME: &str = "atlas_cache_manifest.json";
pub const MANIFEST_VERSION: u32 = 1;

/// Normalized UV coordinate rectangle and pixel bounds within a texture atlas.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpriteUvRect {
    pub sprite_id: String,
    pub atlas_id: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
}

/// Metadata descriptor for a single atlas texture page.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AtlasPageDescriptor {
    pub id: u32,
    pub width: u32,
    pub height: u32,
    pub sprite_count: usize,
}

/// Persistent manifest caching packed sprite UVs and atlas metadata across runs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AtlasCacheManifest {
    pub version: u32,
    pub max_atlas_size: u32,
    pub total_sprites: usize,
    pub pages: Vec<AtlasPageDescriptor>,
    pub sprites: HashMap<String, SpriteUvRect>,
    pub checksum: u64,
}

/// Input sprite dimension for packing.
#[derive(Debug, Clone)]
pub struct SpriteInput {
    pub id: String,
    pub width: u32,
    pub height: u32,
}

struct Shelf {
    y: u32,
    height: u32,
    current_x: u32,
    remaining_width: u32,
}

struct WorkingPage {
    id: u32,
    used_height: u32,
    shelves: Vec<Shelf>,
    sprite_count: usize,
}

/// Manager for texture atlas bin-packing and cache persistence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AtlasCacheManager {
    max_atlas_size: u32,
}

impl Default for AtlasCacheManager {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_ATLAS_SIZE)
    }
}

impl AtlasCacheManager {
    /// Creates a new `AtlasCacheManager` with the specified power-of-two max atlas size.
    pub fn new(max_atlas_size: u32) -> Self {
        let size = if max_atlas_size.is_power_of_two() && max_atlas_size >= 64 {
            max_atlas_size
        } else {
            DEFAULT_MAX_ATLAS_SIZE
        };

        Self {
            max_atlas_size: size,
        }
    }

    /// Gets the configured maximum atlas dimension.
    pub fn max_atlas_size(&self) -> u32 {
        self.max_atlas_size
    }

    /// Packs sprites into one or more power-of-two atlases using a shelf bin-packing heuristic.
    pub fn pack_sprites(
        &self,
        items: &[SpriteInput],
        checksum: u64,
    ) -> Result<AtlasCacheManifest, CoreError> {
        let max_dim = self.max_atlas_size;

        // Check for dimensions exceeding max atlas size
        for item in items {
            if item.width > max_dim || item.height > max_dim {
                return Err(CoreError::Cache(format!(
                    "Sprite '{}' dimensions ({}, {}) exceed maximum atlas size ({})",
                    item.id, item.width, item.height, max_dim
                )));
            }
        }

        // Sort sprites by height descending (and width descending) for optimal packing
        let mut sorted_items: Vec<&SpriteInput> = items.iter().collect();
        sorted_items.sort_by(|a, b| {
            b.height
                .cmp(&a.height)
                .then_with(|| b.width.cmp(&a.width))
                .then_with(|| a.id.cmp(&b.id))
        });

        let mut pages: Vec<WorkingPage> = Vec::new();
        let mut sprite_map: HashMap<String, SpriteUvRect> = HashMap::with_capacity(items.len());

        for item in sorted_items {
            let mut placed = false;

            // 1. Try to fit into existing shelves in existing pages (Best Height Fit)
            for page in &mut pages {
                let mut best_shelf_idx: Option<usize> = None;
                let mut best_waste = u32::MAX;

                for (idx, shelf) in page.shelves.iter().enumerate() {
                    if shelf.remaining_width >= item.width && shelf.height >= item.height {
                        let waste = shelf.height - item.height;
                        if waste < best_waste {
                            best_waste = waste;
                            best_shelf_idx = Some(idx);
                        }
                    }
                }

                if let Some(s_idx) = best_shelf_idx {
                    let shelf = &mut page.shelves[s_idx];
                    let x = shelf.current_x;
                    let y = shelf.y;

                    shelf.current_x += item.width;
                    shelf.remaining_width -= item.width;
                    page.sprite_count += 1;

                    let u0 = x as f32 / max_dim as f32;
                    let v0 = y as f32 / max_dim as f32;
                    let u1 = (x + item.width) as f32 / max_dim as f32;
                    let v1 = (y + item.height) as f32 / max_dim as f32;

                    sprite_map.insert(
                        item.id.clone(),
                        SpriteUvRect {
                            sprite_id: item.id.clone(),
                            atlas_id: page.id,
                            x,
                            y,
                            width: item.width,
                            height: item.height,
                            u0,
                            v0,
                            u1,
                            v1,
                        },
                    );
                    placed = true;
                    break;
                }

                // 2. Try creating a new shelf on this page if height permits
                if !placed && page.used_height + item.height <= max_dim {
                    let y = page.used_height;
                    let shelf_h = item.height;
                    let x = 0;

                    page.shelves.push(Shelf {
                        y,
                        height: shelf_h,
                        current_x: item.width,
                        remaining_width: max_dim - item.width,
                    });
                    page.used_height += shelf_h;
                    page.sprite_count += 1;

                    let u0 = 0.0;
                    let v0 = y as f32 / max_dim as f32;
                    let u1 = item.width as f32 / max_dim as f32;
                    let v1 = (y + item.height) as f32 / max_dim as f32;

                    sprite_map.insert(
                        item.id.clone(),
                        SpriteUvRect {
                            sprite_id: item.id.clone(),
                            atlas_id: page.id,
                            x,
                            y,
                            width: item.width,
                            height: item.height,
                            u0,
                            v0,
                            u1,
                            v1,
                        },
                    );
                    placed = true;
                    break;
                }
            }

            // 3. If still not placed, allocate a new atlas page
            if !placed {
                let new_page_id = pages.len() as u32;
                let y = 0;
                let shelf_h = item.height;
                let x = 0;

                let new_page = WorkingPage {
                    id: new_page_id,
                    used_height: shelf_h,
                    shelves: vec![Shelf {
                        y,
                        height: shelf_h,
                        current_x: item.width,
                        remaining_width: max_dim - item.width,
                    }],
                    sprite_count: 1,
                };

                let u0 = 0.0;
                let v0 = 0.0;
                let u1 = item.width as f32 / max_dim as f32;
                let v1 = item.height as f32 / max_dim as f32;

                sprite_map.insert(
                    item.id.clone(),
                    SpriteUvRect {
                        sprite_id: item.id.clone(),
                        atlas_id: new_page.id,
                        x,
                        y,
                        width: item.width,
                        height: item.height,
                        u0,
                        v0,
                        u1,
                        v1,
                    },
                );

                pages.push(new_page);
            }
        }

        let page_descriptors: Vec<AtlasPageDescriptor> = pages
            .into_iter()
            .map(|p| AtlasPageDescriptor {
                id: p.id,
                width: max_dim,
                height: max_dim,
                sprite_count: p.sprite_count,
            })
            .collect();

        Ok(AtlasCacheManifest {
            version: MANIFEST_VERSION,
            max_atlas_size: max_dim,
            total_sprites: items.len(),
            pages: page_descriptors,
            sprites: sprite_map,
            checksum,
        })
    }

    /// Saves the manifest to `<cache_dir>/atlas_cache_manifest.json`.
    pub fn save_manifest(
        &self,
        manifest: &AtlasCacheManifest,
        cache_dir: &Path,
    ) -> Result<PathBuf, CoreError> {
        fs::create_dir_all(cache_dir)?;
        let manifest_path = cache_dir.join(MANIFEST_FILENAME);
        let json = serde_json::to_string_pretty(manifest)?;
        let mut file = File::create(&manifest_path)?;
        file.write_all(json.as_bytes())?;
        file.flush()?;
        Ok(manifest_path)
    }

    /// Loads the manifest from `<cache_dir>/atlas_cache_manifest.json`.
    pub fn load_manifest(&self, cache_dir: &Path) -> Result<AtlasCacheManifest, CoreError> {
        let manifest_path = cache_dir.join(MANIFEST_FILENAME);
        if !manifest_path.exists() {
            return Err(CoreError::Cache(format!(
                "Manifest file not found: {:?}",
                manifest_path
            )));
        }

        let mut file = File::open(&manifest_path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        let manifest: AtlasCacheManifest = serde_json::from_str(&content)?;

        if manifest.version != MANIFEST_VERSION {
            return Err(CoreError::Cache(format!(
                "Unsupported manifest version: {}",
                manifest.version
            )));
        }

        Ok(manifest)
    }

    /// Validates whether a cached manifest exists and matches the expected package checksum.
    pub fn is_manifest_valid(&self, cache_dir: &Path, expected_checksum: u64) -> bool {
        match self.load_manifest(cache_dir) {
            Ok(m) => m.checksum == expected_checksum,
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shelf_bin_packing_and_manifest_roundtrip() {
        let manager = AtlasCacheManager::new(2048);
        assert_eq!(manager.max_atlas_size(), 2048);

        // Create 100 test sprites with various dimensions
        let mut sprites = Vec::new();
        for i in 0..100 {
            sprites.push(SpriteInput {
                id: format!("spr_pokemon_{:03}", i),
                width: 64 + (i % 64),
                height: 64 + ((i * 3) % 64),
            });
        }

        let checksum = 0x123456789abcdef0;
        let manifest = manager.pack_sprites(&sprites, checksum).expect("Packing failed");

        assert_eq!(manifest.total_sprites, 100);
        assert!(!manifest.pages.is_empty());
        assert_eq!(manifest.checksum, checksum);

        // Verify all sprites are packed and valid UV coordinates in [0.0, 1.0]
        for sprite in &sprites {
            let uv = manifest.sprites.get(&sprite.id).expect("Sprite must exist in manifest");
            assert_eq!(uv.width, sprite.width);
            assert_eq!(uv.height, sprite.height);
            assert!(uv.u0 >= 0.0 && uv.u0 <= 1.0);
            assert!(uv.v0 >= 0.0 && uv.v0 <= 1.0);
            assert!(uv.u1 >= uv.u0 && uv.u1 <= 1.0);
            assert!(uv.v1 >= uv.v0 && uv.v1 <= 1.0);
        }

        // Test saving and loading manifest to temporary directory
        let temp_dir = std::env::temp_dir().join(format!("reloaded_cache_test_{}", std::process::id()));
        let saved_path = manager.save_manifest(&manifest, &temp_dir).expect("Save manifest");
        assert!(saved_path.exists());

        let loaded = manager.load_manifest(&temp_dir).expect("Load manifest");
        assert_eq!(loaded, manifest);
        assert!(manager.is_manifest_valid(&temp_dir, checksum));
        assert!(!manager.is_manifest_valid(&temp_dir, 0xdeadbeef));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_oversized_sprite_rejected() {
        let manager = AtlasCacheManager::new(512);
        let sprites = vec![SpriteInput {
            id: "huge_sprite".to_string(),
            width: 1024,
            height: 64,
        }];

        let result = manager.pack_sprites(&sprites, 1);
        assert!(result.is_err());
    }
}
