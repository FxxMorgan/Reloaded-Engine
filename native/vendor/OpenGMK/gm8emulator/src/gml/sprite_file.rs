//! GameMaker sprite files: magic and compressed length, then a nameless zlib sprite.
//! Based on the format supported by KliseePokemonReloadedAndroid. Validate sizes
//! before allocating or passing pixels to OpenGL.

use byteorder::{ReadBytesExt, LE};
use gm8exe::asset::{sprite::{CollisionMap, Frame}, Sprite};
use std::io::{self, Cursor, Read};

const MAGIC: u32 = 0x0012_D591;
const MAX_INFLATED_BYTES: u64 = 128 * 1024 * 1024;

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn remaining(reader: &Cursor<&[u8]>) -> usize {
    reader.get_ref().len().saturating_sub(reader.position() as usize)
}

fn pixels_len(width: u32, height: u32) -> io::Result<usize> {
    (width as usize)
        .checked_mul(height as usize)
        .and_then(|count| count.checked_mul(4))
        .ok_or_else(|| invalid("sprite dimensions overflow"))
}

fn read_frame(reader: &mut Cursor<&[u8]>) -> io::Result<Frame> {
    if reader.read_u32::<LE>()? != 800 {
        return Err(invalid("unsupported sprite frame version"));
    }
    let width = reader.read_u32::<LE>()?;
    let height = reader.read_u32::<LE>()?;
    let len = reader.read_u32::<LE>()? as usize;
    if width == 0 || height == 0 || width > i32::MAX as u32 || height > i32::MAX as u32
        || len != pixels_len(width, height)? || len > remaining(reader)
    {
        return Err(invalid("invalid or truncated sprite pixels"));
    }
    let start = reader.position() as usize;
    let mut data = reader.get_ref()[start..start + len].to_vec();
    reader.set_position((start + len) as u64);
    // External GM sprites carry BGRA; the renderer consumes RGBA.
    crate::util::bgra2rgba(&mut data);
    Ok(Frame { width, height, data: data.into_boxed_slice() })
}

fn read_collider(reader: &mut Cursor<&[u8]>) -> io::Result<CollisionMap> {
    // sprite_save files from Reloaded use 0 here, while executable assets use 800.
    let version = reader.read_u32::<LE>()?;
    if version != 0 && version != 800 {
        return Err(invalid("unsupported sprite collision version"));
    }
    let width = reader.read_u32::<LE>()?;
    let height = reader.read_u32::<LE>()?;
    let bbox_left = reader.read_u32::<LE>()?;
    let bbox_right = reader.read_u32::<LE>()?;
    let bbox_bottom = reader.read_u32::<LE>()?;
    let bbox_top = reader.read_u32::<LE>()?;
    let len = pixels_len(width, height)?;
    // The shipped files sometimes omit mask pixels even for larger placeholder
    // dimensions. Keep the bbox and use no precise mask, as the reference does.
    let missing_mask = len > remaining(reader);
    let mut data = Vec::with_capacity(if missing_mask { 0 } else { len / 4 });
    if missing_mask {
        reader.set_position(reader.get_ref().len() as u64);
    } else {
        for _ in 0..len / 4 {
            data.push(reader.read_u32::<LE>()? != 0);
        }
    }
    Ok(CollisionMap { width, height, bbox_left, bbox_right, bbox_top, bbox_bottom, data: data.into_boxed_slice() })
}

pub(crate) fn decode_gmspr(bytes: &[u8]) -> Result<Sprite, String> {
    let mut header = Cursor::new(bytes);
    let magic = header.read_u32::<LE>().map_err(|e| e.to_string())?;
    let compressed_len = header.read_u32::<LE>().map_err(|e| e.to_string())? as usize;
    if magic != MAGIC {
        return Err(format!("invalid .gmspr magic {magic:#x}"));
    }
    if compressed_len != bytes.len() - 8 {
        return Err("invalid .gmspr compressed length".into());
    }
    let mut inflated = Vec::new();
    flate2::read::ZlibDecoder::new(&bytes[8..])
        .take(MAX_INFLATED_BYTES + 1)
        .read_to_end(&mut inflated)
        .map_err(|e| format!("cannot decompress .gmspr: {e}"))?;
    if inflated.len() as u64 > MAX_INFLATED_BYTES {
        return Err(".gmspr exceeds decoded size limit".into());
    }
    parse_payload(&inflated).map_err(|e| format!("invalid .gmspr payload: {e}"))
}

fn parse_payload(bytes: &[u8]) -> io::Result<Sprite> {
    let mut reader = Cursor::new(bytes);
    let version = reader.read_u32::<LE>()?;
    if version != 800 && version != 810 {
        return Err(invalid("unsupported sprite version"));
    }
    let origin_x = reader.read_i32::<LE>()?;
    let origin_y = reader.read_i32::<LE>()?;
    let frame_count = reader.read_u32::<LE>()? as usize;
    if frame_count > remaining(&reader) / 16 {
        return Err(invalid("invalid sprite frame count"));
    }
    let mut frames = Vec::with_capacity(frame_count);
    for _ in 0..frame_count {
        frames.push(read_frame(&mut reader)?);
    }
    let mut colliders: Vec<CollisionMap> = Vec::new();
    let per_frame_colliders = if frame_count == 0 {
        false
    } else {
        if version == 810 {
            let _ = reader.read_u32::<LE>(); // Collision shape is represented by the masks.
        }
        let separate = reader.read_u32::<LE>().unwrap_or(0) != 0;
        for index in 0..if separate { frame_count } else { 1 } {
            let collider = read_collider(&mut reader).unwrap_or_else(|_| {
                reader.set_position(bytes.len() as u64);
                let first = colliders.first();
                let frame = &frames[index];
                CollisionMap {
                    width: 0, height: 0,
                    bbox_left: first.map(|c| c.bbox_left).unwrap_or(0),
                    bbox_right: first.map(|c| c.bbox_right).unwrap_or(frame.width - 1),
                    bbox_top: first.map(|c| c.bbox_top).unwrap_or(0),
                    bbox_bottom: first.map(|c| c.bbox_bottom).unwrap_or(frame.height - 1),
                    data: Vec::new().into_boxed_slice(),
                }
            });
            colliders.push(collider);
        }
        separate
    };
    Ok(Sprite { name: "<gmspr>".into(), origin_x, origin_y, frames, colliders, per_frame_colliders })
}

#[cfg(test)]
mod tests {
    use super::*;
    use gm8exe::asset::Asset;
    use std::io::Write;

    fn wrap(payload: &[u8]) -> Vec<u8> {
        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(payload).unwrap();
        let body = encoder.finish().unwrap();
        let mut bytes = MAGIC.to_le_bytes().to_vec();
        bytes.extend_from_slice(&(body.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&body);
        bytes
    }

    fn fixture(separate: bool) -> Vec<u8> {
        let count = if separate { 2 } else { 1 };
        let sprite = Sprite {
            name: Default::default(), origin_x: -3, origin_y: 7,
            frames: (0..count).map(|_| Frame { width: 1, height: 1, data: vec![7, 11, 19, 255].into_boxed_slice() }).collect(),
            colliders: (0..count).map(|_| CollisionMap {
                width: 1, height: 1, bbox_left: 0, bbox_right: 0, bbox_top: 0, bbox_bottom: 0,
                data: vec![true].into_boxed_slice(),
            }).collect(),
            per_frame_colliders: separate,
        };
        let mut serialized = Vec::new();
        sprite.serialize_exe(&mut serialized, gm8exe::GameVersion::GameMaker8_0).unwrap();
        wrap(&serialized[4..]) // External sprites omit the name.
    }

    #[test]
    fn decodes_origins_pixels_and_shared_mask() {
        let sprite = decode_gmspr(&fixture(false)).unwrap();
        assert_eq!((sprite.origin_x, sprite.origin_y), (-3, 7));
        assert_eq!(sprite.frames[0].data.as_ref(), &[19, 11, 7, 255]);
        assert_eq!(sprite.colliders[0].data.as_ref(), &[true]);
        assert!(!sprite.per_frame_colliders);
    }

    #[test]
    fn preserves_individual_frames_and_masks() {
        let sprite = decode_gmspr(&fixture(true)).unwrap();
        assert_eq!(sprite.frames.len(), 2);
        assert_eq!(sprite.colliders.len(), 2);
        assert!(sprite.per_frame_colliders);
    }

    #[test]
    fn rejects_bad_headers_and_truncated_files() {
        assert!(decode_gmspr(&[]).is_err());
        let mut bytes = fixture(false);
        bytes[0] ^= 1;
        assert!(decode_gmspr(&bytes).is_err());
        let mut bytes = fixture(false);
        bytes.pop();
        assert!(decode_gmspr(&bytes).is_err());
    }

    #[test]
    fn rejects_pixel_size_mismatch_without_allocating_image() {
        let mut payload = Vec::new();
        for value in [800u32, 0, 0, 1, 800, u32::MAX, u32::MAX, 4] {
            payload.extend_from_slice(&value.to_le_bytes());
        }
        payload.extend_from_slice(&[0; 4]);
        assert!(decode_gmspr(&wrap(&payload)).is_err());
    }

    #[test]
    fn accepts_reloadeds_zero_version_empty_collision_mask() {
        // Structure of data/pkmnbattle/P1_2.gmspr: no mask pixels, but a valid bbox.
        let mut payload = Vec::new();
        for value in [800u32, 9, 11, 1, 800, 19, 22, 19 * 22 * 4] {
            payload.extend_from_slice(&value.to_le_bytes());
        }
        payload.extend_from_slice(&vec![0; 19 * 22 * 4]);
        for value in [0u32, 0, 0, 0, 0, 18, 21, 0] {
            payload.extend_from_slice(&value.to_le_bytes());
        }
        let sprite = decode_gmspr(&wrap(&payload)).unwrap();
        assert_eq!((sprite.frames[0].width, sprite.frames[0].height), (19, 22));
        assert_eq!((sprite.colliders[0].bbox_right, sprite.colliders[0].bbox_bottom), (18, 21));
        assert!(sprite.colliders[0].data.is_empty());
    }

    #[test]
    fn accepts_missing_per_frame_placeholder_masks() {
        // Structure of P373_A.gmspr: five frames, one 1x1 header, no mask pixels.
        let mut payload = Vec::new();
        for value in [800u32, 24, 20, 5] {
            payload.extend_from_slice(&value.to_le_bytes());
        }
        for _ in 0..5 {
            for value in [800u32, 48, 40, 48 * 40 * 4] {
                payload.extend_from_slice(&value.to_le_bytes());
            }
            payload.extend_from_slice(&vec![0; 48 * 40 * 4]);
        }
        for value in [1u32, 0, 1, 1, 0, 47, 39, 0] {
            payload.extend_from_slice(&value.to_le_bytes());
        }
        let sprite = decode_gmspr(&wrap(&payload)).unwrap();
        assert!(sprite.per_frame_colliders);
        assert_eq!(sprite.colliders.len(), 5);
        for collider in sprite.colliders {
            assert!(collider.data.iter().all(|&pixel| !pixel));
            assert_eq!((collider.bbox_right, collider.bbox_bottom), (47, 39));
        }
    }

    #[test]
    fn incomplete_masks_preserve_the_sprite_and_its_bbox() {
        for tail in [vec![], vec![0u32, 0, 2, 3, 0, 1, 2, 0]] {
            let mut payload = Vec::new();
            for value in [800u32, 0, 0, 1, 800, 2, 3, 24] {
                payload.extend_from_slice(&value.to_le_bytes());
            }
            payload.extend_from_slice(&[0; 24]);
            for value in tail {
                payload.extend_from_slice(&value.to_le_bytes());
            }
            let sprite = decode_gmspr(&wrap(&payload)).unwrap();
            assert_eq!(sprite.frames.len(), 1);
            assert_eq!((sprite.colliders[0].bbox_right, sprite.colliders[0].bbox_bottom), (1, 2));
            assert!(sprite.colliders[0].data.is_empty());
        }
    }
}
