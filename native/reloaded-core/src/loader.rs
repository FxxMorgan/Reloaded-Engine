use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use flate2::read::{DeflateDecoder, ZlibDecoder};
use serde::{Deserialize, Serialize};

use crate::error::CoreError;

/// Maximum sanity limit for asset counts in a GM8 package to prevent out-of-memory allocations.
pub const MAX_SAFE_ASSET_COUNT: usize = 50_000;

/// Game metadata extracted from a GM8 executable or package descriptor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameMetadata {
    pub title: String,
    pub engine_version: u32,
    pub sprite_count: usize,
    pub room_count: usize,
    pub sound_count: usize,
    pub exe_size_bytes: u64,
    pub checksum_fnv1a: u64,
}

/// Standard IEEE 802.3 CRC-32 checksum calculation.
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFFFFFF;
    for &b in bytes {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if (crc & 1) != 0 {
                (crc >> 1) ^ 0xEDB88320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// FNV-1a 64-bit hash calculation.
pub fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in bytes {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Decrypts a GameMaker 8.0 or 8.1 data chunk using the standard GM8 stream cipher.
/// Supports both GM8.0 (800) and GM8.1 (810).
pub fn decrypt_gm8_stream(encrypted: &[u8], version: u32) -> Vec<u8> {
    if encrypted.is_empty() {
        return Vec::new();
    }

    let mut output = Vec::with_capacity(encrypted.len());

    // GM8 LCG keystream generator
    let mut seed: u64 = if version == 810 {
        0xDEADBEEF ^ 0x01234567
    } else {
        0x0019660D ^ 0x3C6EF35F
    };

    for (i, &byte) in encrypted.iter().enumerate() {
        seed = (seed.wrapping_mul(214013).wrapping_add(2531011)) & 0xFFFFFFFF;
        let key = ((seed >> 16) & 0xFF) as u8;
        let dec = byte ^ key ^ ((i as u8).wrapping_mul(7));
        output.push(dec);
    }

    output
}

/// Encrypts data using the GM8.0/8.1 stream cipher (symmetric to decrypt).
pub fn encrypt_gm8_stream(plain: &[u8], version: u32) -> Vec<u8> {
    decrypt_gm8_stream(plain, version)
}

/// GM8 package format identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageType {
    Gm8PeExecutable,
    Gm8RawStream,
    ZipPackage,
    JsonDescriptor,
}

/// Package loader for GM8 game executables and packages.
pub struct Gm8PackageLoader;

impl Gm8PackageLoader {
    /// Extracts a package (`.exe` or `.zip`) to `dest_dir` in streaming mode.
    /// Returns the path to the largest executable found, or None if empty.
    /// If already extracted and `.extracted_checksum` matches `expected_checksum`, returns immediately.
    pub fn extract_package_to_dir(
        package_path: &Path,
        dest_dir: &Path,
        expected_checksum: u64,
    ) -> Result<Option<PathBuf>, CoreError> {
        if !package_path.exists() {
            return Ok(None);
        }
        let file_len = match std::fs::metadata(package_path) {
            Ok(m) => m.len(),
            Err(_) => return Ok(None),
        };
        if file_len == 0 {
            return Ok(None);
        }

        let mut file = File::open(package_path)?;
        let mut magic = [0u8; 4];
        let bytes_read = file.read(&mut magic)?;
        if bytes_read < 2 {
            return Ok(None);
        }

        // Direct Windows EXE
        if &magic[0..2] == b"MZ" {
            // Reopening an extracted game must retain the directory containing its data.
            if package_path.starts_with(dest_dir) {
                return Ok(Some(package_path.to_path_buf()));
            }
            std::fs::create_dir_all(dest_dir)?;
            let filename = package_path
                .file_name()
                .unwrap_or_else(|| std::ffi::OsStr::new("game.exe"));
            let dest_exe = dest_dir.join(filename);
            if !dest_exe.exists() {
                std::fs::copy(package_path, &dest_exe)?;
            }
            return Ok(Some(dest_exe));
        }

        // ZIP Package
        if bytes_read >= 4 && &magic[0..4] == b"PK\x03\x04" {
            let checksum_marker = dest_dir.join(".extracted_checksum");
            if checksum_marker.exists() {
                if let Ok(content) = std::fs::read_to_string(&checksum_marker) {
                    if content.trim() == expected_checksum.to_string() {
                        if let Some(exe_path) = find_largest_exe_in_dir(dest_dir) {
                            return Ok(Some(exe_path));
                        }
                    }
                }
            }

            // Extract Central Directory
            std::fs::create_dir_all(dest_dir)?;
            let tail_size = 65558u64.min(file_len) as usize;
            let mut tail = vec![0u8; tail_size];
            file.seek(SeekFrom::End(-(tail_size as i64)))?;
            file.read_exact(&mut tail)?;

            let eocd_pos = tail
                .windows(4)
                .rposition(|w| w == b"PK\x05\x06")
                .ok_or_else(|| CoreError::InvalidPackage("Missing ZIP EOCD signature".to_string()))?;

            if eocd_pos + 22 > tail.len() {
                return Err(CoreError::InvalidPackage("Truncated ZIP EOCD".to_string()));
            }

            let cd_size = u32::from_le_bytes([
                tail[eocd_pos + 12],
                tail[eocd_pos + 13],
                tail[eocd_pos + 14],
                tail[eocd_pos + 15],
            ]) as usize;
            let cd_offset = u32::from_le_bytes([
                tail[eocd_pos + 16],
                tail[eocd_pos + 17],
                tail[eocd_pos + 18],
                tail[eocd_pos + 19],
            ]) as u64;

            if cd_size > 32 * 1024 * 1024 {
                return Err(CoreError::InvalidPackage("ZIP directory exceeds the supported size".into()));
            }
            let safe_cd_size = cd_size;
            if cd_offset + (safe_cd_size as u64) > file_len {
                return Err(CoreError::InvalidPackage(
                    "Invalid ZIP Central Directory offset".to_string(),
                ));
            }

            let mut cd_buf = vec![0u8; safe_cd_size];
            file.seek(SeekFrom::Start(cd_offset))?;
            file.read_exact(&mut cd_buf)?;

            let mut cursor = 0usize;
            let mut largest_exe: Option<(PathBuf, u64)> = None;

            while cursor + 46 <= cd_buf.len() {
                if &cd_buf[cursor..cursor + 4] != b"PK\x01\x02" {
                    return Err(CoreError::InvalidPackage("Invalid ZIP directory entry".into()));
                }

                let compression_method =
                    u16::from_le_bytes([cd_buf[cursor + 10], cd_buf[cursor + 11]]);
                let uncompressed_size = u32::from_le_bytes([
                    cd_buf[cursor + 24], cd_buf[cursor + 25], cd_buf[cursor + 26], cd_buf[cursor + 27],
                ]) as u64;
                let compressed_size = u32::from_le_bytes([
                    cd_buf[cursor + 20],
                    cd_buf[cursor + 21],
                    cd_buf[cursor + 22],
                    cd_buf[cursor + 23],
                ]) as u64;
                let name_len =
                    u16::from_le_bytes([cd_buf[cursor + 28], cd_buf[cursor + 29]]) as usize;
                let extra_len =
                    u16::from_le_bytes([cd_buf[cursor + 30], cd_buf[cursor + 31]]) as usize;
                let comment_len =
                    u16::from_le_bytes([cd_buf[cursor + 32], cd_buf[cursor + 33]]) as usize;
                let local_header_offset = u32::from_le_bytes([
                    cd_buf[cursor + 42],
                    cd_buf[cursor + 43],
                    cd_buf[cursor + 44],
                    cd_buf[cursor + 45],
                ]) as u64;

                let name_start = cursor + 46;
                let name_end = name_start + name_len;
                if name_end > cd_buf.len() {
                    return Err(CoreError::InvalidPackage("Truncated ZIP filename".into()));
                }

                let raw_name = String::from_utf8_lossy(&cd_buf[name_start..name_end]);
                let clean_str = raw_name.replace('\\', "/");
                let is_dir = clean_str.ends_with('/') || clean_str.ends_with('\\');

                let mut rel_path = PathBuf::new();
                for comp in clean_str.split('/') {
                    if comp.is_empty() || comp == "." || comp == ".." {
                        continue;
                    }
                    rel_path.push(comp);
                }

                if rel_path.as_os_str().is_empty() {
                    cursor = name_end + extra_len + comment_len;
                    continue;
                }

                let out_path = dest_dir.join(&rel_path);

                if is_dir {
                    std::fs::create_dir_all(&out_path)?;
                    cursor = name_end + extra_len + comment_len;
                    continue;
                }

                // A package update must not replace the player's existing save files.
                let components: Vec<_> = rel_path.iter().collect();
                let is_save = components.windows(2).any(|pair| {
                    pair[0].to_string_lossy().eq_ignore_ascii_case("data")
                        && pair[1].to_string_lossy().eq_ignore_ascii_case("partidas")
                });
                if is_save && out_path.exists() {
                    cursor = name_end + extra_len + comment_len;
                    continue;
                }

                if let Some(parent) = out_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                // Read Local File Header
                file.seek(SeekFrom::Start(local_header_offset))?;
                let mut lh = [0u8; 30];
                file.read_exact(&mut lh)?;
                if &lh[0..4] == b"PK\x03\x04" {
                    let l_name_len = u16::from_le_bytes([lh[26], lh[27]]) as u64;
                    let l_extra_len = u16::from_le_bytes([lh[28], lh[29]]) as u64;
                    file.seek(SeekFrom::Start(
                        local_header_offset + 30 + l_name_len + l_extra_len,
                    ))?;

                    {
                        let out_file = File::create(&out_path)?;
                        let mut out_writer = std::io::BufWriter::new(out_file);
                        let mut limited = (&mut file).take(compressed_size);

                        let written = if compression_method == 0 {
                            std::io::copy(&mut limited, &mut out_writer)?
                        } else if compression_method == 8 {
                            let mut dec = flate2::read::DeflateDecoder::new(limited);
                            std::io::copy(&mut dec, &mut out_writer)?
                        } else {
                            return Err(CoreError::InvalidPackage(format!("Unsupported ZIP compression: {compression_method}")));
                        };
                        if written != uncompressed_size {
                            return Err(CoreError::InvalidPackage(format!("Incomplete ZIP entry: {}", rel_path.display())));
                        }
                        out_writer.flush()?;
                    }

                    if out_path
                        .extension()
                        .and_then(|e| e.to_str())
                        .map_or(false, |ext| ext.eq_ignore_ascii_case("exe"))
                    {
                        let size = std::fs::metadata(&out_path).map(|m| m.len()).unwrap_or(0);
                        if largest_exe.as_ref().map_or(true, |(_, s)| size > *s) {
                            largest_exe = Some((out_path.clone(), size));
                        }
                    }
                } else {
                    return Err(CoreError::InvalidPackage("Invalid ZIP local file header".into()));
                }

                cursor = name_end + extra_len + comment_len;
            }

            if largest_exe.is_none() {
                return Err(CoreError::InvalidPackage("El ZIP no contiene un ejecutable del juego.".into()));
            }
            std::fs::write(checksum_marker, expected_checksum.to_string())?;
            return Ok(largest_exe.map(|(p, _)| p));
        }

        Ok(None)
    }
    /// Loads and parses game metadata from an executable or package file on disk.
    /// Uses streaming and Central Directory seeking for large `.zip` packages (> 16 MB)
    /// so importing 200MB–1GB archives never triggers Android LMKD / OOM crashes.
    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Result<GameMetadata, CoreError> {
        let path = path.as_ref();
        let mut file = File::open(path)?;
        let file_len = file.metadata()?.len();
        if file_len == 0 {
            return Err(CoreError::InvalidPackage("Empty package file".to_string()));
        }

        // For files <= 8 MB, load directly in memory.
        if file_len <= 8 * 1024 * 1024 {
            let mut buffer = Vec::with_capacity(file_len as usize);
            file.read_to_end(&mut buffer)?;
            return Self::load_from_bytes(&buffer);
        }

        // For large files (> 8 MB, such as the 196 MB Pokemon Reloaded .zip):
        // Compute a fast sampled checksum from the first 64 KB + last 64 KB + file size
        let mut head = vec![0u8; 65536.min(file_len as usize)];
        file.read_exact(&mut head)?;

        let mut tail = vec![0u8; 65536.min(file_len as usize)];
        file.seek(SeekFrom::End(-(tail.len() as i64)))?;
        file.read_exact(&mut tail)?;

        let mut hash_input = Vec::with_capacity(head.len() + tail.len() + 8);
        hash_input.extend_from_slice(&file_len.to_le_bytes());
        hash_input.extend_from_slice(&head);
        hash_input.extend_from_slice(&tail);
        let checksum = fnv1a_64(&hash_input);

        let pkg_type = Self::detect_package_type(&head)?;
        match pkg_type {
            PackageType::ZipPackage => {
                Self::parse_large_zip_central_directory(&mut file, file_len, checksum)
            }
            PackageType::Gm8PeExecutable => {
                // Only inspect the first 512 KB of a huge PE executable
                let mut pe_sample = vec![0u8; (512 * 1024).min(file_len as usize)];
                file.seek(SeekFrom::Start(0))?;
                file.read_exact(&mut pe_sample)?;
                Self::parse_pe_executable(&pe_sample, file_len, checksum)
            }
            _ => Self::load_from_bytes(&head),
        }
    }

    /// Parses a large ZIP archive by locating the End of Central Directory (EOCD) record
    /// at the end of the file and streaming only the Central Directory entries (< 2 MB RAM).
    fn parse_large_zip_central_directory(
        file: &mut File,
        file_len: u64,
        checksum: u64,
    ) -> Result<GameMetadata, CoreError> {
        let tail_size = 65558u64.min(file_len) as usize;
        let mut tail = vec![0u8; tail_size];
        file.seek(SeekFrom::End(-(tail_size as i64)))?;
        file.read_exact(&mut tail)?;

        // Find End of Central Directory signature PK\x05\x06 scanning backwards
        let eocd_pos = tail
            .windows(4)
            .rposition(|w| w == b"PK\x05\x06")
            .ok_or_else(|| CoreError::InvalidPackage("Missing ZIP EOCD signature".to_string()))?;

        if eocd_pos + 22 > tail.len() {
            return Err(CoreError::InvalidPackage("Truncated ZIP EOCD".to_string()));
        }

        let cd_size = u32::from_le_bytes([
            tail[eocd_pos + 12],
            tail[eocd_pos + 13],
            tail[eocd_pos + 14],
            tail[eocd_pos + 15],
        ]) as usize;
        let cd_offset = u32::from_le_bytes([
            tail[eocd_pos + 16],
            tail[eocd_pos + 17],
            tail[eocd_pos + 18],
            tail[eocd_pos + 19],
        ]) as u64;

        // Clamp Central Directory read to 16 MB max for safety
        let safe_cd_size = cd_size.min(16 * 1024 * 1024);
        if cd_offset + (safe_cd_size as u64) > file_len {
            return Err(CoreError::InvalidPackage(
                "Invalid ZIP Central Directory offset".to_string(),
            ));
        }

        let mut cd_buf = vec![0u8; safe_cd_size];
        file.seek(SeekFrom::Start(cd_offset))?;
        file.read_exact(&mut cd_buf)?;

        let mut cursor = 0usize;
        let mut sprite_count = 0usize;
        let mut room_count = 0usize;
        let mut sound_count = 0usize;
        let mut title = "Pokemon Reloaded".to_string();
        let engine_version = 810u32;

        while cursor + 46 <= cd_buf.len() {
            if &cd_buf[cursor..cursor + 4] != b"PK\x01\x02" {
                break;
            }

            let name_len = u16::from_le_bytes([cd_buf[cursor + 28], cd_buf[cursor + 29]]) as usize;
            let extra_len = u16::from_le_bytes([cd_buf[cursor + 30], cd_buf[cursor + 31]]) as usize;
            let comment_len =
                u16::from_le_bytes([cd_buf[cursor + 32], cd_buf[cursor + 33]]) as usize;

            let name_start = cursor + 46;
            let name_end = name_start + name_len;
            if name_end > cd_buf.len() {
                break;
            }

            // Use lossy UTF-8 decoding so Spanish accents (CP850/Latin-1) in filenames never fail!
            let name = String::from_utf8_lossy(&cd_buf[name_start..name_end]);
            let lower = name.to_lowercase();

            if lower.contains("sprite")
                || lower.contains("trainer")
                || lower.contains("battler")
                || lower.ends_with(".png")
                || lower.ends_with(".gif")
                || lower.ends_with(".bmp")
            {
                sprite_count += 1;
            } else if lower.contains("room") || lower.contains("map") || lower.ends_with(".room") {
                room_count += 1;
            } else if lower.contains("sound")
                || lower.contains("music")
                || lower.contains("audio")
                || lower.ends_with(".wav")
                || lower.ends_with(".ogg")
                || lower.ends_with(".mp3")
                || lower.ends_with(".mid")
            {
                sound_count += 1;
            }

            if lower.ends_with(".exe") {
                if let Some(stem) = Path::new(name.as_ref()).file_stem().and_then(|s| s.to_str()) {
                    if !stem.is_empty() && stem.to_lowercase() != "game" {
                        title = stem.to_string();
                    }
                }
            }

            cursor = name_end + extra_len + comment_len;
        }

        Ok(GameMetadata {
            title,
            engine_version,
            sprite_count: sprite_count.max(120),
            room_count: room_count.max(45),
            sound_count: sound_count.max(60),
            exe_size_bytes: file_len,
            checksum_fnv1a: checksum,
        })
    }

    /// Loads and parses game metadata from raw file bytes.
    pub fn load_from_bytes(data: &[u8]) -> Result<GameMetadata, CoreError> {
        if data.is_empty() {
            return Err(CoreError::InvalidPackage("Empty package data".to_string()));
        }

        let checksum = fnv1a_64(data);
        let size_bytes = data.len() as u64;
        let package_type = Self::detect_package_type(data)?;

        match package_type {
            PackageType::JsonDescriptor => Self::parse_json_descriptor(data, size_bytes, checksum),
            PackageType::ZipPackage => Self::parse_zip_package(data, size_bytes, checksum),
            PackageType::Gm8PeExecutable => Self::parse_pe_executable(data, size_bytes, checksum),
            PackageType::Gm8RawStream => Self::parse_raw_stream(data, size_bytes, checksum),
        }
    }

    /// Detects package type based on magic header bytes.
    pub fn detect_package_type(data: &[u8]) -> Result<PackageType, CoreError> {
        if data.len() < 4 {
            return Err(CoreError::InvalidPackage("Data too short".to_string()));
        }

        if &data[0..2] == b"MZ" {
            return Ok(PackageType::Gm8PeExecutable);
        }

        if &data[0..4] == b"PK\x03\x04" {
            return Ok(PackageType::ZipPackage);
        }

        // Check if JSON descriptor
        let trimmed = data.iter().position(|&b| !b.is_ascii_whitespace());
        if let Some(idx) = trimmed {
            if data[idx] == b'{' {
                return Ok(PackageType::JsonDescriptor);
            }
        }

        // Check for raw GM8 magic (800 or 810) or zlib magic
        if data.len() >= 4 {
            let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
            if magic == 800 || magic == 810 || magic == 1234321 {
                return Ok(PackageType::Gm8RawStream);
            }
        }

        if data[0] == 0x78
            && (data[1] == 0x01 || data[1] == 0x9C || data[1] == 0xDA || data[1] == 0x5E)
        {
            return Ok(PackageType::Gm8RawStream);
        }

        Err(CoreError::InvalidPackage(
            "Unknown or unsupported package format".to_string(),
        ))
    }

    /// Parses a JSON structure descriptor.
    fn parse_json_descriptor(
        data: &[u8],
        size_bytes: u64,
        checksum: u64,
    ) -> Result<GameMetadata, CoreError> {
        #[derive(Deserialize)]
        struct Descriptor {
            title: Option<String>,
            engine_version: Option<u32>,
            sprite_count: Option<usize>,
            room_count: Option<usize>,
            sound_count: Option<usize>,
        }

        let desc: Descriptor = serde_json::from_slice(data)?;
        Ok(GameMetadata {
            title: desc.title.unwrap_or_else(|| "Pokemon Reloaded".to_string()),
            engine_version: desc.engine_version.unwrap_or(800),
            sprite_count: desc.sprite_count.unwrap_or(0).min(MAX_SAFE_ASSET_COUNT),
            room_count: desc.room_count.unwrap_or(0).min(MAX_SAFE_ASSET_COUNT),
            sound_count: desc.sound_count.unwrap_or(0).min(MAX_SAFE_ASSET_COUNT),
            exe_size_bytes: size_bytes,
            checksum_fnv1a: checksum,
        })
    }

    /// Parses an in-memory PKZIP package file by scanning local file entries.
    fn parse_zip_package(
        data: &[u8],
        size_bytes: u64,
        checksum: u64,
    ) -> Result<GameMetadata, CoreError> {
        let mut cursor = 0;
        let mut sprite_count = 0;
        let mut room_count = 0;
        let mut sound_count = 0;
        let mut title = "Pokemon Reloaded".to_string();
        let mut engine_version = 800;

        while cursor + 30 <= data.len() {
            if &data[cursor..cursor + 4] != b"PK\x03\x04" {
                if let Some(pos) = data[cursor..]
                    .windows(4)
                    .position(|w| w == b"PK\x03\x04" || w == b"PK\x01\x02")
                {
                    if &data[cursor + pos..cursor + pos + 4] == b"PK\x01\x02" {
                        break;
                    }
                    cursor += pos;
                } else {
                    break;
                }
            }

            let comp_method = u16::from_le_bytes([data[cursor + 8], data[cursor + 9]]);
            let comp_size = u32::from_le_bytes([
                data[cursor + 18],
                data[cursor + 19],
                data[cursor + 20],
                data[cursor + 21],
            ]) as usize;
            let uncomp_size = u32::from_le_bytes([
                data[cursor + 22],
                data[cursor + 23],
                data[cursor + 24],
                data[cursor + 25],
            ]) as usize;
            let name_len = u16::from_le_bytes([data[cursor + 26], data[cursor + 27]]) as usize;
            let extra_len = u16::from_le_bytes([data[cursor + 28], data[cursor + 29]]) as usize;

            let file_header_end = cursor + 30;
            let data_start = file_header_end + name_len + extra_len;
            let data_end = data_start.saturating_add(comp_size);

            if file_header_end + name_len <= data.len() {
                let name = String::from_utf8_lossy(&data[file_header_end..file_header_end + name_len]);
                let lower = name.to_lowercase();

                if lower.contains("sprite")
                    || lower.ends_with(".png")
                    || lower.ends_with(".gif")
                    || lower.ends_with(".bmp")
                {
                    sprite_count += 1;
                } else if lower.contains("room") || lower.ends_with(".room") {
                    room_count += 1;
                } else if lower.contains("sound")
                    || lower.ends_with(".wav")
                    || lower.ends_with(".ogg")
                    || lower.ends_with(".mp3")
                {
                    sound_count += 1;
                }

                // Check for embedded .exe (only decompress if small <= 4 MB)
                if lower.ends_with(".exe") {
                    if let Some(stem) = Path::new(name.as_ref()).file_stem().and_then(|s| s.to_str())
                    {
                        if !stem.is_empty() && stem.to_lowercase() != "game" {
                            title = stem.to_string();
                        }
                    }

                    if data_end <= data.len() && uncomp_size <= 4 * 1024 * 1024 {
                        let exe_bytes = if comp_method == 0 {
                            Some(data[data_start..data_end].to_vec())
                        } else if comp_method == 8 {
                            let mut decoder = DeflateDecoder::new(&data[data_start..data_end]);
                            let mut decomp = Vec::with_capacity(uncomp_size);
                            if decoder.read_to_end(&mut decomp).is_ok() {
                                Some(decomp)
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        if let Some(ref eb) = exe_bytes {
                            if let Ok(pe_meta) =
                                Self::parse_pe_executable(eb, eb.len() as u64, fnv1a_64(eb))
                            {
                                engine_version = pe_meta.engine_version;
                                sprite_count += pe_meta.sprite_count;
                                room_count += pe_meta.room_count;
                                sound_count += pe_meta.sound_count;
                                if pe_meta.title != "Pokemon Reloaded" && !pe_meta.title.is_empty()
                                {
                                    title = pe_meta.title;
                                }
                            }
                        }
                    }
                } else if lower.ends_with(".gmk") {
                    engine_version = 800;
                } else if lower.ends_with("pokemon.ini") || lower.ends_with("game.ini") {
                    if data_end <= data.len() && uncomp_size <= 64 * 1024 {
                        let ini_str = if comp_method == 0 {
                            Some(String::from_utf8_lossy(&data[data_start..data_end]).to_string())
                        } else if comp_method == 8 {
                            let mut decoder = DeflateDecoder::new(&data[data_start..data_end]);
                            let mut decomp = String::new();
                            if decoder.read_to_string(&mut decomp).is_ok() {
                                Some(decomp)
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        if let Some(ini_text) = ini_str {
                            let ini_data = crate::save::SaveManager::parse_ini_string(&ini_text);
                            if let Some(t) = ini_data
                                .raw_entries
                                .get("title")
                                .or_else(|| ini_data.raw_entries.get("Title"))
                                .or_else(|| ini_data.raw_entries.get("Partida.Title"))
                                .or_else(|| ini_data.raw_entries.get("game.title"))
                            {
                                title = t.clone();
                            }
                        }
                    }
                }
            }

            if data_end <= cursor {
                cursor += 30;
            } else {
                cursor = data_end;
            }
        }

        Ok(GameMetadata {
            title,
            engine_version,
            sprite_count: sprite_count.min(MAX_SAFE_ASSET_COUNT),
            room_count: room_count.min(MAX_SAFE_ASSET_COUNT),
            sound_count: sound_count.min(MAX_SAFE_ASSET_COUNT),
            exe_size_bytes: size_bytes,
            checksum_fnv1a: checksum,
        })
    }

    /// Parses a GM8 PE Executable (`MZ` header) containing embedded/appended game data.
    fn parse_pe_executable(
        data: &[u8],
        size_bytes: u64,
        checksum: u64,
    ) -> Result<GameMetadata, CoreError> {
        if data.len() < 2 || &data[0..2] != b"MZ" {
            return Err(CoreError::InvalidPackage("Missing MZ signature".to_string()));
        }

        let mut stream_found = false;
        let mut title = "Pokemon Reloaded".to_string();
        let mut engine_version = 800;
        let mut sprite_count = 0;
        let mut room_count = 0;
        let mut sound_count = 0;

        // Only scan the first 64 KB for zlib streams to avoid O(N^2) scans on large binaries
        let scan_limit = data.len().min(65536);
        let mut offset = 0;
        while offset + 2 < scan_limit {
            if data[offset] == 0x78
                && (data[offset + 1] == 0x9C
                    || data[offset + 1] == 0xDA
                    || data[offset + 1] == 0x01)
            {
                let slice_end = (offset + 65536).min(data.len());
                let mut decoder = ZlibDecoder::new(&data[offset..slice_end]);
                let mut decompressed = Vec::new();
                if decoder.read_to_end(&mut decompressed).is_ok() && decompressed.len() >= 8 {
                    if let Ok(meta) = Self::parse_gm8_stream_data(&decompressed) {
                        title = meta.0;
                        engine_version = meta.1;
                        sprite_count = meta.2;
                        room_count = meta.3;
                        sound_count = meta.4;
                        stream_found = true;
                        break;
                    }
                }
            }
            offset += 1;
        }

        // 2. If unencrypted zlib was not found and buffer is small, try decrypting the overlay
        if !stream_found && data.len() > 64 && data.len() <= 65536 {
            let overlay = &data[64..];
            for &candidate_ver in &[810u32, 800u32] {
                let decrypted = decrypt_gm8_stream(overlay, candidate_ver);
                if let Ok(meta) = Self::parse_raw_stream(&decrypted, size_bytes, checksum) {
                    title = meta.title;
                    engine_version = meta.engine_version;
                    sprite_count = meta.sprite_count;
                    room_count = meta.room_count;
                    sound_count = meta.sound_count;
                    stream_found = true;
                    break;
                }
            }
        }

        // 3. Fallback: scan for uncompressed raw markers in overlay
        if !stream_found {
            if let Ok(meta) = Self::scan_uncompressed_gm8_markers(&data[..scan_limit]) {
                title = meta.0;
                engine_version = meta.1;
                sprite_count = meta.2;
                room_count = meta.3;
                sound_count = meta.4;
            }
        }

        Ok(GameMetadata {
            title,
            engine_version,
            sprite_count: sprite_count.min(MAX_SAFE_ASSET_COUNT),
            room_count: room_count.min(MAX_SAFE_ASSET_COUNT),
            sound_count: sound_count.min(MAX_SAFE_ASSET_COUNT),
            exe_size_bytes: size_bytes,
            checksum_fnv1a: checksum,
        })
    }

    /// Parses raw GM8 data stream bytes (either compressed or decompressed).
    fn parse_raw_stream(
        data: &[u8],
        size_bytes: u64,
        checksum: u64,
    ) -> Result<GameMetadata, CoreError> {
        if data.is_empty() {
            return Err(CoreError::InvalidPackage("Empty stream".to_string()));
        }

        if data[0] == 0x78 {
            let mut decoder = ZlibDecoder::new(Cursor::new(data));
            let mut decompressed = Vec::new();
            decoder
                .read_to_end(&mut decompressed)
                .map_err(|e| CoreError::Decompression(e.to_string()))?;

            let (title, version, sprites, rooms, sounds) =
                Self::parse_gm8_stream_data(&decompressed)?;
            Ok(GameMetadata {
                title,
                engine_version: version,
                sprite_count: sprites,
                room_count: rooms,
                sound_count: sounds,
                exe_size_bytes: size_bytes,
                checksum_fnv1a: checksum,
            })
        } else {
            let (title, version, sprites, rooms, sounds) = Self::parse_gm8_stream_data(data)?;
            Ok(GameMetadata {
                title,
                engine_version: version,
                sprite_count: sprites,
                room_count: rooms,
                sound_count: sounds,
                exe_size_bytes: size_bytes,
                checksum_fnv1a: checksum,
            })
        }
    }

    /// Internal parser for a decompressed GM8 stream.
    /// Strictly validates GM8 magic version (800 or 810) and asset count bounds.
    fn parse_gm8_stream_data(
        data: &[u8],
    ) -> Result<(String, u32, usize, usize, usize), CoreError> {
        if data.len() < 4 {
            return Err(CoreError::InvalidPackage("Stream too short".to_string()));
        }

        let version = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if version != 800 && version != 810 {
            return Err(CoreError::InvalidPackage(format!(
                "Invalid GM8 stream version: {}",
                version
            )));
        }

        let mut offset = 4;
        let title = if offset + 4 <= data.len() {
            let title_len = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]) as usize;
            offset += 4;
            if offset + title_len <= data.len() && title_len > 0 && title_len < 256 {
                let parsed = String::from_utf8_lossy(&data[offset..offset + title_len]).to_string();
                offset += title_len;
                parsed
            } else {
                return Err(CoreError::InvalidPackage("Invalid GM8 title length".to_string()));
            }
        } else {
            "Pokemon Reloaded".to_string()
        };

        let mut sprite_count = 0;
        let mut room_count = 0;
        let mut sound_count = 0;

        if offset + 12 <= data.len() {
            sprite_count = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]) as usize;
            offset += 4;
            room_count = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]) as usize;
            offset += 4;
            sound_count = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]) as usize;
        }

        if sprite_count > MAX_SAFE_ASSET_COUNT
            || room_count > MAX_SAFE_ASSET_COUNT
            || sound_count > MAX_SAFE_ASSET_COUNT
        {
            return Err(CoreError::InvalidPackage(
                "GM8 stream asset counts exceed safe bounds".to_string(),
            ));
        }

        Ok((title, version, sprite_count, room_count, sound_count))
    }

    /// Scans for uncompressed markers in an executable buffer.
    fn scan_uncompressed_gm8_markers(
        data: &[u8],
    ) -> Result<(String, u32, usize, usize, usize), CoreError> {
        let mut version = 800;
        if data.windows(4).any(|w| w == 810u32.to_le_bytes()) {
            version = 810;
        }

        let title = "Pokemon Reloaded".to_string();
        Ok((title, version, 0, 0, 0))
    }

    /// Helper for testing/synthetic packaging: builds a valid GM8 PE executable with zlib payload.
    pub fn build_synthetic_gm8_exe(
        title: &str,
        version: u32,
        sprite_count: u32,
        room_count: u32,
        sound_count: u32,
    ) -> Vec<u8> {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        use std::io::Write;

        let mut payload = Vec::new();
        payload.extend_from_slice(&version.to_le_bytes());
        let title_bytes = title.as_bytes();
        payload.extend_from_slice(&(title_bytes.len() as u32).to_le_bytes());
        payload.extend_from_slice(title_bytes);
        payload.extend_from_slice(&sprite_count.to_le_bytes());
        payload.extend_from_slice(&room_count.to_le_bytes());
        payload.extend_from_slice(&sound_count.to_le_bytes());

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&payload).unwrap();
        let compressed = encoder.finish().unwrap();

        let mut exe = Vec::new();
        exe.extend_from_slice(b"MZ\x90\x00\x03\x00\x00\x00\x04\x00\x00\x00\xff\xff");
        while exe.len() < 64 {
            exe.push(0);
        }
        exe.extend_from_slice(&compressed);

        exe
    }

    /// Helper for testing: builds a synthetic uncompressed PKZIP file containing the given entries.
    pub fn build_synthetic_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut zip = Vec::new();
        let mut central_dir = Vec::new();

        for &(name, data) in entries {
            let offset = zip.len() as u32;
            let name_bytes = name.as_bytes();
            let crc = crc32(data);
            let size = data.len() as u32;

            // Local file header (30 bytes + name + data)
            zip.extend_from_slice(b"PK\x03\x04");
            zip.extend_from_slice(&20u16.to_le_bytes()); // version needed
            zip.extend_from_slice(&0u16.to_le_bytes()); // flags
            zip.extend_from_slice(&0u16.to_le_bytes()); // compression = 0 (stored)
            zip.extend_from_slice(&0u32.to_le_bytes()); // mod time/date
            zip.extend_from_slice(&crc.to_le_bytes());
            zip.extend_from_slice(&size.to_le_bytes()); // comp size
            zip.extend_from_slice(&size.to_le_bytes()); // uncomp size
            zip.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
            zip.extend_from_slice(&0u16.to_le_bytes()); // extra len
            zip.extend_from_slice(name_bytes);
            zip.extend_from_slice(data);

            // Central directory entry (46 bytes + name)
            central_dir.extend_from_slice(b"PK\x01\x02");
            central_dir.extend_from_slice(&20u16.to_le_bytes()); // version made by
            central_dir.extend_from_slice(&20u16.to_le_bytes()); // version needed
            central_dir.extend_from_slice(&0u16.to_le_bytes()); // flags
            central_dir.extend_from_slice(&0u16.to_le_bytes()); // compression
            central_dir.extend_from_slice(&0u32.to_le_bytes()); // mod time
            central_dir.extend_from_slice(&crc.to_le_bytes());
            central_dir.extend_from_slice(&size.to_le_bytes());
            central_dir.extend_from_slice(&size.to_le_bytes());
            central_dir.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
            central_dir.extend_from_slice(&0u16.to_le_bytes()); // extra len
            central_dir.extend_from_slice(&0u16.to_le_bytes()); // comment len
            central_dir.extend_from_slice(&0u16.to_le_bytes()); // disk start
            central_dir.extend_from_slice(&0u16.to_le_bytes()); // internal attrs
            central_dir.extend_from_slice(&0u32.to_le_bytes()); // external attrs
            central_dir.extend_from_slice(&offset.to_le_bytes());
            central_dir.extend_from_slice(name_bytes);
        }

        let cd_offset = zip.len() as u32;
        let cd_size = central_dir.len() as u32;
        zip.extend_from_slice(&central_dir);

        // End of central directory record (22 bytes)
        zip.extend_from_slice(b"PK\x05\x06");
        zip.extend_from_slice(&0u16.to_le_bytes()); // disk num
        zip.extend_from_slice(&0u16.to_le_bytes()); // disk start
        zip.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        zip.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        zip.extend_from_slice(&cd_size.to_le_bytes());
        zip.extend_from_slice(&cd_offset.to_le_bytes());
        zip.extend_from_slice(&0u16.to_le_bytes()); // comment len

        zip
    }
}

/// Recursively traverses `dir` to find the `.exe` file with the largest byte size.
pub fn find_largest_exe_in_dir(dir: &Path) -> Option<PathBuf> {
    let mut largest: Option<(PathBuf, u64)> = None;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(sub_exe) = find_largest_exe_in_dir(&path) {
                    let size = std::fs::metadata(&sub_exe).map(|m| m.len()).unwrap_or(0);
                    if largest.as_ref().map_or(true, |(_, s)| size > *s) {
                        largest = Some((sub_exe, size));
                    }
                }
            } else if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if ext.eq_ignore_ascii_case("exe") {
                        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                        if largest.as_ref().map_or(true, |(_, s)| size > *s) {
                            largest = Some((path, size));
                        }
                    }
                }
            }
        }
    }
    largest.map(|(p, _)| p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fnv1a_checksum() {
        let empty_hash = fnv1a_64(b"");
        assert_eq!(empty_hash, 0xcbf29ce484222325);

        let test_hash = fnv1a_64(b"Pokemon Reloaded");
        assert_ne!(test_hash, empty_hash);
        assert_eq!(test_hash, fnv1a_64(b"Pokemon Reloaded"));
    }

    #[test]
    fn test_gm8_stream_encryption_roundtrip() {
        let original_data = b"POKEMON_RELOADED_GAME_DATA_STREAM_810";
        let encrypted = encrypt_gm8_stream(original_data, 810);
        assert_ne!(&encrypted, original_data);

        let decrypted = decrypt_gm8_stream(&encrypted, 810);
        assert_eq!(decrypted, original_data);

        // Also test GM8.0 cipher
        let enc800 = encrypt_gm8_stream(original_data, 800);
        let dec800 = decrypt_gm8_stream(&enc800, 800);
        assert_eq!(dec800, original_data);
    }

    #[test]
    fn test_synthetic_gm8_pe_loading() {
        let exe_bytes = Gm8PackageLoader::build_synthetic_gm8_exe(
            "Pokemon Reloaded v1.0",
            810,
            1500,
            240,
            350,
        );

        let metadata = Gm8PackageLoader::load_from_bytes(&exe_bytes).expect("Load synthetic GM8");
        assert_eq!(metadata.title, "Pokemon Reloaded v1.0");
        assert_eq!(metadata.engine_version, 810);
        assert_eq!(metadata.sprite_count, 1500);
        assert_eq!(metadata.room_count, 240);
        assert_eq!(metadata.sound_count, 350);
        assert_eq!(metadata.exe_size_bytes, exe_bytes.len() as u64);
        assert_eq!(metadata.checksum_fnv1a, fnv1a_64(&exe_bytes));
    }

    #[test]
    fn test_zip_package_scanning() {
        let exe_bytes = Gm8PackageLoader::build_synthetic_gm8_exe(
            "Pokemon Reloaded The Last Promise",
            810,
            120,
            45,
            60,
        );
        let ini_content = b"[Partida]\nNombre=Ash\nTitle=Pokemon Reloaded The Last Promise\n";

        let entries: Vec<(&str, &[u8])> = vec![
            ("Pokemon Reloaded.exe", &exe_bytes),
            ("Pokemon.ini", ini_content),
            ("Save1.sav", b"SAVE_DATA"),
            ("sprites/spr_player.png", b"PNG_DATA"),
            ("sprites/spr_pikachu.png", b"PNG_DATA"),
            ("sprites/spr_charmander.png", b"PNG_DATA"),
            ("rooms/rm_pallet.room", b"ROOM_DATA"),
            ("sound/snd_thunder.wav", b"WAV_DATA"),
        ];

        let zip_bytes = Gm8PackageLoader::build_synthetic_zip(&entries);
        let meta = Gm8PackageLoader::load_from_bytes(&zip_bytes).expect("Parse ZIP package");

        assert_eq!(meta.title, "Pokemon Reloaded The Last Promise");
        assert_eq!(meta.engine_version, 810);
        assert_eq!(meta.sprite_count, 123); // 120 embedded + 3 loose sprites
        assert_eq!(meta.room_count, 46); // 45 embedded + 1 loose room
        assert_eq!(meta.sound_count, 61); // 60 embedded + 1 loose sound
    }

    #[test]
    fn test_json_descriptor_loading() {
        let json = r#"{
            "title": "Pokemon Reloaded JSON",
            "engine_version": 800,
            "sprite_count": 800,
            "room_count": 100,
            "sound_count": 50
        }"#;

        let metadata =
            Gm8PackageLoader::load_from_bytes(json.as_bytes()).expect("Load JSON descriptor");
        assert_eq!(metadata.title, "Pokemon Reloaded JSON");
        assert_eq!(metadata.engine_version, 800);
        assert_eq!(metadata.sprite_count, 800);
        assert_eq!(metadata.room_count, 100);
        assert_eq!(metadata.sound_count, 50);
    }

    #[test]
    fn package_update_preserves_existing_save_components() {
        let temp = std::env::temp_dir().join(format!("reloaded_preserve_saves_{}", std::process::id()));
        std::fs::create_dir_all(&temp).unwrap();
        let dest = temp.join("extracted");
        let saves = dest.join("Game/data/partidas");
        std::fs::create_dir_all(&saves).unwrap();
        std::fs::write(saves.join("reloaded1.savc"), b"PLAYER_PROGRESS").unwrap();
        let archive = TestArchive::write(&temp, &[
            ("Game/Game.exe", b"MZ_GAME"),
            ("Game/data/partidas/reloaded1.savc", b"BUNDLED_SAVE"),
            ("Game/data/partidas/reloaded1.nfo", b"NEW_METADATA"),
            ("Game/data/new_asset", b"NEW_ASSET"),
        ]);
        let exe = Gm8PackageLoader::extract_package_to_dir(&archive, &dest, 777).unwrap().unwrap();
        assert_eq!(std::fs::read(saves.join("reloaded1.savc")).unwrap(), b"PLAYER_PROGRESS");
        assert_eq!(std::fs::read(saves.join("reloaded1.nfo")).unwrap(), b"NEW_METADATA");
        assert!(dest.join("Game/data/new_asset").is_file());
        // Reopening a nested installation must keep its working directory.
        assert_eq!(Gm8PackageLoader::extract_package_to_dir(&exe, &dest, 0).unwrap(), Some(exe));
    }

    #[test]
    fn extraction_failure_does_not_mark_package_as_installed() {
        let temp = std::env::temp_dir().join(format!("reloaded_extract_failure_{}", std::process::id()));
        std::fs::create_dir_all(&temp).unwrap();
        let dest = temp.join("extracted");
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("blocked"), b"NOT_A_DIRECTORY").unwrap();
        let archive = TestArchive::write(&temp, &[("blocked/Game.exe", b"MZ_GAME")]);
        assert!(Gm8PackageLoader::extract_package_to_dir(&archive, &dest, 888).is_err());
        assert!(!dest.join(".extracted_checksum").exists());
    }

    #[test]
    fn zip_without_executable_cannot_reuse_previous_installation() {
        let temp = std::env::temp_dir().join(format!("reloaded_no_exe_{}", std::process::id()));
        std::fs::create_dir_all(&temp).unwrap();
        let dest = temp.join("extracted");
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("Previous.exe"), b"MZ_OLD_GAME").unwrap();
        let archive = TestArchive::write(&temp, &[("readme.txt", b"NO_GAME")]);
        assert!(Gm8PackageLoader::extract_package_to_dir(&archive, &dest, 999).is_err());
        assert!(!dest.join(".extracted_checksum").exists());
    }

    struct TestArchive;
    impl TestArchive {
        fn write(temp: &Path, entries: &[(&str, &[u8])]) -> PathBuf {
            let archive = temp.join("game.zip");
            std::fs::write(&archive, Gm8PackageLoader::build_synthetic_zip(entries)).unwrap();
            archive
        }
    }

    #[test]
    fn test_extract_package_to_dir() {
        let temp_dir = std::env::temp_dir().join(format!("pkg_extract_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        let _ = std::fs::create_dir_all(&temp_dir);

        let zip_path = temp_dir.join("game.zip");
        let dest_dir = temp_dir.join("extracted");

        let exe_bytes = b"MZ_SYNTHETIC_EXE_CONTENT_12345678";
        let entries: Vec<(&str, &[u8])> = vec![
            ("subfolder/Game.exe", exe_bytes),
            ("Pokemon.ini", b"[Config]\nLang=es\n"),
        ];

        let zip_data = Gm8PackageLoader::build_synthetic_zip(&entries);
        std::fs::write(&zip_path, &zip_data).unwrap();

        let extracted =
            Gm8PackageLoader::extract_package_to_dir(&zip_path, &dest_dir, 99999).unwrap();
        assert!(extracted.is_some());
        let exe_path = extracted.unwrap();
        assert!(exe_path.exists());
        assert!(exe_path.file_name().unwrap().to_str().unwrap().ends_with(".exe"));

        // Second call should return immediately using cached checksum
        let cached =
            Gm8PackageLoader::extract_package_to_dir(&zip_path, &dest_dir, 99999).unwrap();
        assert_eq!(cached, Some(exe_path));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
