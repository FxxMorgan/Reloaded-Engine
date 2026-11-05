use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::audio::LockFreeAudioMixer;
use crate::input::{
    InputSnapshot, BTN_A, BTN_C, BTN_D, BTN_DOWN, BTN_F1, BTN_F2, BTN_F3, BTN_F4, BTN_F5,
    BTN_LEFT, BTN_RIGHT, BTN_S, BTN_UP, BTN_X, BTN_Z,
};
use crate::renderer::{Gles3BatchRenderer, SpriteQuad};

/// Dynamic value type used in GameMaker Language (GML) scripts and instance variables.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GmlValue {
    Real(f64),
    String(String),
    Boolean(bool),
    Array(Vec<GmlValue>),
    Undefined,
}

impl GmlValue {
    pub fn as_f32(&self) -> f32 {
        match self {
            GmlValue::Real(v) => *v as f32,
            GmlValue::Boolean(b) => if *b { 1.0 } else { 0.0 },
            _ => 0.0,
        }
    }

    pub fn as_bool(&self) -> bool {
        match self {
            GmlValue::Boolean(b) => *b,
            GmlValue::Real(v) => *v != 0.0,
            _ => false,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            GmlValue::String(s) => s.as_str(),
            _ => "",
        }
    }
}

/// Standard GM8 event lifecycle indicators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GmlEvent {
    Create,
    Destroy,
    Step,
    Collision,
    Draw,
    Keyboard,
}

/// Represents an active entity / object instance in a GM8 room.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Gm8Instance {
    pub id: u32,
    pub object_name: String,
    pub sprite_atlas_id: u32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub hspeed: f32,
    pub vspeed: f32,
    pub depth: i32,
    pub color: [f32; 4],
    pub visible: bool,
    pub vars: HashMap<String, GmlValue>,
}

impl Gm8Instance {
    /// Returns the Axis-Aligned Bounding Box (min_x, min_y, max_x, max_y).
    #[inline]
    pub fn bounding_box(&self) -> (f32, f32, f32, f32) {
        (self.x, self.y, self.x + self.width, self.y + self.height)
    }

    /// Checks if this instance collides with another instance using AABB overlap.
    #[inline]
    pub fn collides_with(&self, other: &Gm8Instance) -> bool {
        let (ax1, ay1, ax2, ay2) = self.bounding_box();
        let (bx1, by1, bx2, by2) = other.bounding_box();

        ax1 < bx2 && ax2 > bx1 && ay1 < by2 && ay2 > by1
    }
}

/// A GameMaker 8 room containing instances and dimensions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Gm8Room {
    pub id: u32,
    pub name: String,
    pub width: f32,
    pub height: f32,
    pub instances: Vec<Gm8Instance>,
    pub tile_size: f32,
}

/// Synthesizes short procedural SFX waveforms for interactive battle/field feedback.
fn generate_synth_sfx(base_freq: f32, freq_slide: f32, duration_secs: f32, volume: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let total_samples = (sample_rate * duration_secs) as usize;
    let mut buffer = Vec::with_capacity(total_samples * 2);

    let mut phase = 0.0f32;
    for i in 0..total_samples {
        let t = i as f32 / sample_rate;
        let progress = t / duration_secs;
        let current_freq = base_freq + freq_slide * progress;
        phase += 2.0 * std::f32::consts::PI * current_freq / sample_rate;

        // Exponential decay envelope
        let env = (1.0 - progress) * (1.0 - progress);
        let s = phase.sin() * env * volume;

        // Interleaved stereo [L, R]
        buffer.push(s);
        buffer.push(s);
    }

    buffer
}

/// GameMaker 8 Room & GML Virtual Machine.
/// Drives the interactive *Pokémon Reloaded* simulation in real time.
pub struct GmlVm {
    pub current_room: Gm8Room,
    pub next_instance_id: u32,
    pub bike_active: bool,
    pub player_id: u32,
    pub frame_count: u64,
}

impl Default for GmlVm {
    fn default() -> Self {
        Self::new_interactive_room(800.0, 480.0)
    }
}

impl GmlVm {
    /// Creates a new `GmlVm` loaded with an interactive Pokémon Reloaded starter/battle/overworld room.
    pub fn new_interactive_room(width: f32, height: f32) -> Self {
        let mut instances = Vec::new();

        // 1. Player Trainer / Pokémon Instance (depth 0, atlas 1)
        let mut player_vars = HashMap::new();
        player_vars.insert("hp".to_string(), GmlValue::Real(100.0));
        player_vars.insert("max_hp".to_string(), GmlValue::Real(100.0));
        player_vars.insert("is_player".to_string(), GmlValue::Boolean(true));

        let player = Gm8Instance {
            id: 1,
            object_name: "obj_player_trainer".to_string(),
            sprite_atlas_id: 1,
            x: 200.0,
            y: 200.0,
            width: 32.0,
            height: 32.0,
            hspeed: 0.0,
            vspeed: 0.0,
            depth: 0,
            color: [1.0, 1.0, 1.0, 1.0],
            visible: true,
            vars: player_vars,
        };
        instances.push(player);

        // 2. Rival / Wild Pokémon Target 1 (depth 10, atlas 1, autonomous patrol)
        let mut wild1_vars = HashMap::new();
        wild1_vars.insert("hp".to_string(), GmlValue::Real(60.0));
        wild1_vars.insert("max_hp".to_string(), GmlValue::Real(60.0));
        wild1_vars.insert("hit_flash".to_string(), GmlValue::Real(0.0));
        wild1_vars.insert("is_target".to_string(), GmlValue::Boolean(true));

        let wild1 = Gm8Instance {
            id: 2,
            object_name: "obj_wild_pokemon".to_string(),
            sprite_atlas_id: 1,
            x: 500.0,
            y: 180.0,
            width: 36.0,
            height: 36.0,
            hspeed: 2.0,
            vspeed: 0.0,
            depth: 10,
            color: [0.95, 0.45, 0.45, 1.0], // Salmon/Red wild pokémon tint
            visible: true,
            vars: wild1_vars,
        };
        instances.push(wild1);

        // 3. Rival / Wild Pokémon Target 2 (depth 10, atlas 1)
        let mut wild2_vars = HashMap::new();
        wild2_vars.insert("hp".to_string(), GmlValue::Real(80.0));
        wild2_vars.insert("max_hp".to_string(), GmlValue::Real(80.0));
        wild2_vars.insert("hit_flash".to_string(), GmlValue::Real(0.0));
        wild2_vars.insert("is_target".to_string(), GmlValue::Boolean(true));

        let wild2 = Gm8Instance {
            id: 3,
            object_name: "obj_wild_pokemon".to_string(),
            sprite_atlas_id: 1,
            x: 600.0,
            y: 320.0,
            width: 40.0,
            height: 40.0,
            hspeed: -1.5,
            vspeed: 0.0,
            depth: 10,
            color: [0.45, 0.75, 0.95, 1.0], // Blue wild pokémon tint
            visible: true,
            vars: wild2_vars,
        };
        instances.push(wild2);

        let room = Gm8Room {
            id: 0,
            name: "rm_pallet_overworld".to_string(),
            width,
            height,
            instances,
            tile_size: 32.0,
        };

        Self {
            current_room: room,
            next_instance_id: 100,
            bike_active: false,
            player_id: 1,
            frame_count: 0,
        }
    }

    /// Allocates a unique instance ID.
    fn alloc_id(&mut self) -> u32 {
        let id = self.next_instance_id;
        self.next_instance_id = self.next_instance_id.wrapping_add(1);
        id
    }

    /// Steps the virtual machine logic by 1 tick, reading the current input snapshot
    /// and pumping synthesized audio events.
    pub fn step(&mut self, input: &InputSnapshot, audio: &mut LockFreeAudioMixer) {
        self.frame_count += 1;

        // 1. Process Macro Field Actions (F1 - F12)
        if input.is_pressed(BTN_F1) {
            // Toggle Bike mode
            self.bike_active = !self.bike_active;
            let chime = generate_synth_sfx(880.0, 440.0, 0.12, 0.4);
            audio.play_sfx(chime, 0.7);
        }

        // F2 - F5: Field Radar & Item Beacons
        if input.is_pressed(BTN_F2) {
            // Spawn expanding radar pulse
            let p_x = self.get_player().map(|p| p.x).unwrap_or(200.0);
            let p_y = self.get_player().map(|p| p.y).unwrap_or(200.0);
            let id = self.alloc_id();

            let mut radar_vars = HashMap::new();
            radar_vars.insert("lifetime".to_string(), GmlValue::Real(30.0));
            radar_vars.insert("growth".to_string(), GmlValue::Real(3.0));

            self.current_room.instances.push(Gm8Instance {
                id,
                object_name: "obj_field_radar".to_string(),
                sprite_atlas_id: 0,
                x: p_x - 8.0,
                y: p_y - 8.0,
                width: 48.0,
                height: 48.0,
                hspeed: 0.0,
                vspeed: 0.0,
                depth: -20,
                color: [0.3, 0.8, 1.0, 0.6], // Light cyan ring
                visible: true,
                vars: radar_vars,
            });

            let ping = generate_synth_sfx(1200.0, -300.0, 0.15, 0.35);
            audio.play_sfx(ping, 0.6);
        }

        // F3 - F5: Field Radar & Item Beacons
        if input.is_pressed(BTN_F3) || input.is_pressed(BTN_F4) || input.is_pressed(BTN_F5) {
            let p_x = self.get_player().map(|p| p.x).unwrap_or(200.0);
            let p_y = self.get_player().map(|p| p.y).unwrap_or(200.0);
            let id = self.alloc_id();

            let mut beacon_vars = HashMap::new();
            beacon_vars.insert("lifetime".to_string(), GmlValue::Real(45.0));

            self.current_room.instances.push(Gm8Instance {
                id,
                object_name: "obj_field_beacon".to_string(),
                sprite_atlas_id: 1,
                x: p_x + 16.0,
                y: p_y + 16.0,
                width: 16.0,
                height: 16.0,
                hspeed: 0.0,
                vspeed: 0.0,
                depth: -5,
                color: [0.4, 0.9, 0.6, 0.9],
                visible: true,
                vars: beacon_vars,
            });

            let beacon_sfx = generate_synth_sfx(1050.0, -200.0, 0.1, 0.3);
            audio.play_sfx(beacon_sfx, 0.5);
        }

        // 2. Compute Player Movement & Direction
        let move_speed = if input.is_down(BTN_S) || self.bike_active {
            7.0f32 // Running or Bike speed boost
        } else {
            4.0f32 // Normal walking speed
        };

        let mut dx = 0.0f32;
        let mut dy = 0.0f32;

        if input.is_down(BTN_LEFT) {
            dx -= move_speed;
        }
        if input.is_down(BTN_RIGHT) {
            dx += move_speed;
        }
        if input.is_down(BTN_UP) {
            dy -= move_speed;
        }
        if input.is_down(BTN_DOWN) {
            dy += move_speed;
        }

        let room_w = self.current_room.width;
        let room_h = self.current_room.height;
        let bike = self.bike_active;

        let mut player_pos = (200.0, 200.0);
        if let Some(player) = self.current_room.instances.iter_mut().find(|i| i.id == self.player_id) {
            player.x = (player.x + dx).clamp(0.0, room_w - player.width);
            player.y = (player.y + dy).clamp(0.0, room_h - player.height);
            player_pos = (player.x, player.y);

            // Bike golden visual tint
            player.color = if bike {
                [1.0, 0.85, 0.2, 1.0] // Golden bike tint
            } else {
                [1.0, 1.0, 1.0, 1.0] // Clean default
            };
        }

        // 3. Process Skill & Attack Spawns
        // BTN_Z: Thunderbolt / Electric Shockwave
        if input.is_pressed(BTN_Z) {
            let id = self.alloc_id();
            let mut vars = HashMap::new();
            vars.insert("damage".to_string(), GmlValue::Real(25.0));
            vars.insert("lifetime".to_string(), GmlValue::Real(45.0));

            self.current_room.instances.push(Gm8Instance {
                id,
                object_name: "obj_attack_thunderbolt".to_string(),
                sprite_atlas_id: 1,
                x: player_pos.0 + 32.0,
                y: player_pos.1 + 8.0,
                width: 24.0,
                height: 16.0,
                hspeed: 8.5,
                vspeed: 0.0,
                depth: -10,
                color: [1.0, 0.95, 0.2, 1.0], // Electric Yellow
                visible: true,
                vars,
            });

            let zap = generate_synth_sfx(650.0, 300.0, 0.10, 0.5);
            audio.play_sfx(zap, 0.8);
        }

        // BTN_X: Flamethrower / Fire Blast
        if input.is_pressed(BTN_X) {
            let id = self.alloc_id();
            let mut vars = HashMap::new();
            vars.insert("damage".to_string(), GmlValue::Real(20.0));
            vars.insert("lifetime".to_string(), GmlValue::Real(40.0));

            self.current_room.instances.push(Gm8Instance {
                id,
                object_name: "obj_attack_flame".to_string(),
                sprite_atlas_id: 1,
                x: player_pos.0 + 32.0,
                y: player_pos.1 + 6.0,
                width: 28.0,
                height: 20.0,
                hspeed: 7.0,
                vspeed: 0.0,
                depth: -10,
                color: [1.0, 0.35, 0.1, 1.0], // Fiery Coral Red
                visible: true,
                vars,
            });

            let roar = generate_synth_sfx(320.0, -120.0, 0.14, 0.5);
            audio.play_sfx(roar, 0.8);
        }

        // BTN_C: Water Pulse / Hydro Pump
        if input.is_pressed(BTN_C) {
            let id = self.alloc_id();
            let mut vars = HashMap::new();
            vars.insert("damage".to_string(), GmlValue::Real(18.0));
            vars.insert("lifetime".to_string(), GmlValue::Real(50.0));

            self.current_room.instances.push(Gm8Instance {
                id,
                object_name: "obj_attack_water".to_string(),
                sprite_atlas_id: 1,
                x: player_pos.0 + 32.0,
                y: player_pos.1 + 8.0,
                width: 22.0,
                height: 22.0,
                hspeed: 7.5,
                vspeed: 0.0,
                depth: -10,
                color: [0.25, 0.65, 1.0, 1.0], // Azure Blue
                visible: true,
                vars,
            });

            let splash = generate_synth_sfx(480.0, -180.0, 0.12, 0.45);
            audio.play_sfx(splash, 0.7);
        }

        // BTN_A: Razor Leaf / Grass Cutter
        if input.is_pressed(BTN_A) {
            let id = self.alloc_id();
            let mut vars = HashMap::new();
            vars.insert("damage".to_string(), GmlValue::Real(15.0));
            vars.insert("lifetime".to_string(), GmlValue::Real(45.0));

            self.current_room.instances.push(Gm8Instance {
                id,
                object_name: "obj_attack_leaf".to_string(),
                sprite_atlas_id: 1,
                x: player_pos.0 + 32.0,
                y: player_pos.1 + 10.0,
                width: 20.0,
                height: 14.0,
                hspeed: 8.0,
                vspeed: -0.5,
                depth: -10,
                color: [0.35, 0.85, 0.4, 1.0], // Sage Green
                visible: true,
                vars,
            });

            let slice = generate_synth_sfx(720.0, 200.0, 0.08, 0.4);
            audio.play_sfx(slice, 0.7);
        }

        // BTN_D: Psychic Barrier / Defense Shield
        if input.is_pressed(BTN_D) {
            let id = self.alloc_id();
            let mut vars = HashMap::new();
            vars.insert("lifetime".to_string(), GmlValue::Real(30.0));

            self.current_room.instances.push(Gm8Instance {
                id,
                object_name: "obj_skill_barrier".to_string(),
                sprite_atlas_id: 1,
                x: player_pos.0 - 6.0,
                y: player_pos.1 - 6.0,
                width: 44.0,
                height: 44.0,
                hspeed: 0.0,
                vspeed: 0.0,
                depth: -5,
                color: [0.85, 0.4, 0.95, 0.75], // Lavender Shield
                visible: true,
                vars,
            });

            let shield = generate_synth_sfx(950.0, -400.0, 0.16, 0.4);
            audio.play_sfx(shield, 0.75);
        }

        // 4. Update Autonomous Target / Rival AI
        for inst in &mut self.current_room.instances {
            if inst.object_name == "obj_wild_pokemon" {
                inst.x += inst.hspeed;
                inst.y += inst.vspeed;

                // Ping-pong bounce within overworld boundaries
                if inst.x >= room_w - inst.width - 20.0 {
                    inst.hspeed = -inst.hspeed.abs();
                } else if inst.x <= 320.0 {
                    inst.hspeed = inst.hspeed.abs();
                }

                // Handle hit flash countdown
                let flash = inst.vars.get("hit_flash").map(|v| v.as_f32()).unwrap_or(0.0);
                if flash > 0.0 {
                    inst.vars.insert("hit_flash".to_string(), GmlValue::Real((flash - 1.0) as f64));
                    inst.color = [1.0, 1.0, 1.0, 1.0]; // Bright white hit flash
                } else {
                    // Restore natural target tint
                    if inst.id == 2 {
                        inst.color = [0.95, 0.45, 0.45, 1.0];
                    } else {
                        inst.color = [0.45, 0.75, 0.95, 1.0];
                    }
                }
            } else if inst.object_name.starts_with("obj_attack_") {
                // Move projectiles
                inst.x += inst.hspeed;
                inst.y += inst.vspeed;
            } else if inst.object_name == "obj_field_radar" {
                // Expand radar pulse
                let growth = inst.vars.get("growth").map(|v| v.as_f32()).unwrap_or(2.0);
                inst.width += growth * 2.0;
                inst.height += growth * 2.0;
                inst.x -= growth;
                inst.y -= growth;
            }
        }

        // 5. Collision Detection: Attack Projectiles vs Wild Pokémon Targets
        let mut hit_sfx = false;
        let mut expired_projectile_ids = Vec::new();
        let mut hits = Vec::new();

        let num_instances = self.current_room.instances.len();
        for i in 0..num_instances {
            if !self.current_room.instances[i].object_name.starts_with("obj_attack_") {
                continue;
            }

            let proj = &self.current_room.instances[i];
            let damage = proj.vars.get("damage").map(|v| v.as_f32()).unwrap_or(10.0);

            for j in 0..num_instances {
                if i == j {
                    continue;
                }

                let target = &self.current_room.instances[j];
                if target.object_name == "obj_wild_pokemon" && proj.collides_with(target) {
                    expired_projectile_ids.push(proj.id);
                    hits.push((target.id, damage));
                    hit_sfx = true;
                    break;
                }
            }
        }

        // Apply hit damage & flashes
        for (target_id, damage) in hits {
            if let Some(t_mut) = self.current_room.instances.iter_mut().find(|inst| inst.id == target_id) {
                let cur_hp = t_mut.vars.get("hp").map(|v| v.as_f32()).unwrap_or(50.0);
                t_mut.vars.insert("hp".to_string(), GmlValue::Real((cur_hp - damage).max(0.0) as f64));
                t_mut.vars.insert("hit_flash".to_string(), GmlValue::Real(6.0));
            }
        }

        if hit_sfx {
            let hit = generate_synth_sfx(280.0, -100.0, 0.08, 0.6);
            audio.play_sfx(hit, 0.9);
        }

        // 6. Update lifetimes and prune expired or out-of-bounds instances
        for inst in &mut self.current_room.instances {
            if let Some(life) = inst.vars.get("lifetime").map(|v| v.as_f32()) {
                inst.vars.insert("lifetime".to_string(), GmlValue::Real((life - 1.0) as f64));
            }
        }

        self.current_room.instances.retain(|inst| {
            if expired_projectile_ids.contains(&inst.id) {
                return false;
            }

            if let Some(life) = inst.vars.get("lifetime").map(|v| v.as_f32()) {
                if life <= 0.0 {
                    return false;
                }
            }

            // Prune offscreen projectiles
            if inst.object_name.starts_with("obj_attack_") {
                if inst.x > room_w + 50.0 || inst.x < -50.0 || inst.y > room_h + 50.0 || inst.y < -50.0 {
                    return false;
                }
            }

            true
        });
    }

    /// Emits background tiles and depth-sorted instance quads directly to `renderer`.
    pub fn emit_quads(&self, renderer: &mut Gles3BatchRenderer) {
        let tile_size = self.current_room.tile_size;
        let room_w = self.current_room.width;
        let room_h = self.current_room.height;

        // 1. Emit Tile Grid Background Quads (atlas_id = 0, gentle pleasant grass tones)
        let cols = (room_w / tile_size).ceil() as usize;
        let rows = (room_h / tile_size).ceil() as usize;

        for r in 0..rows {
            for c in 0..cols {
                let x = c as f32 * tile_size;
                let y = r as f32 * tile_size;

                // Checkerboard subtle variation for visual ground texture
                let is_even = (r + c) % 2 == 0;
                let color = if is_even {
                    [0.86, 0.92, 0.86, 1.0] // Soft sage ground
                } else {
                    [0.82, 0.88, 0.82, 1.0] // Slightly darker sage ground
                };

                renderer.submit_quad(SpriteQuad::new(
                    0, // Atlas 0 for background tiles
                    x,
                    y,
                    tile_size,
                    tile_size,
                    0.0,
                    0.0,
                    1.0,
                    1.0,
                    color,
                ));
            }
        }

        // 2. Sort Visible Instances by Depth Descending (higher depth in back, lower depth in front)
        let mut sorted_instances: Vec<&Gm8Instance> = self
            .current_room
            .instances
            .iter()
            .filter(|i| i.visible)
            .collect();

        sorted_instances.sort_by(|a, b| b.depth.cmp(&a.depth).then_with(|| a.id.cmp(&b.id)));

        // 3. Emit Sprite Quads for Entities
        for inst in sorted_instances {
            renderer.submit_quad(SpriteQuad::new(
                inst.sprite_atlas_id,
                inst.x,
                inst.y,
                inst.width,
                inst.height,
                0.0,
                0.0,
                1.0,
                1.0,
                inst.color,
            ));
        }
    }

    /// Helper to find player instance.
    pub fn get_player(&self) -> Option<&Gm8Instance> {
        self.current_room.instances.iter().find(|i| i.id == self.player_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_movement_and_speed_boosts() {
        let mut vm = GmlVm::new_interactive_room(800.0, 480.0);
        let mut audio = LockFreeAudioMixer::new(512);

        let initial_x = vm.get_player().unwrap().x;
        assert_eq!(initial_x, 200.0);

        // Frame 1: Move right normal speed (4.0 px)
        let mut input1 = InputSnapshot::default();
        input1.current = BTN_RIGHT;
        vm.step(&input1, &mut audio);

        let x1 = vm.get_player().unwrap().x;
        assert_eq!(x1, initial_x + 4.0);

        // Frame 2: Move right with Run (BTN_S, 7.0 px)
        let mut input2 = InputSnapshot::default();
        input2.current = BTN_RIGHT | BTN_S;
        vm.step(&input2, &mut audio);

        let x2 = vm.get_player().unwrap().x;
        assert_eq!(x2, x1 + 7.0);

        // Frame 3: Toggle Bike with BTN_F1
        let mut input3 = InputSnapshot::default();
        input3.pressed = BTN_F1;
        input3.current = BTN_RIGHT;
        vm.step(&input3, &mut audio);

        assert!(vm.bike_active);
        let x3 = vm.get_player().unwrap().x;
        assert_eq!(x3, x2 + 7.0); // Bike grants 7.0 px movement without holding BTN_S
        assert_eq!(vm.get_player().unwrap().color, [1.0, 0.85, 0.2, 1.0]); // Golden tint
    }

    #[test]
    fn test_attack_spawning_and_collision() {
        let mut vm = GmlVm::new_interactive_room(800.0, 480.0);
        let mut audio = LockFreeAudioMixer::new(512);

        // Position wild target 1 a bit ahead of player
        if let Some(target) = vm.current_room.instances.iter_mut().find(|i| i.id == 2) {
            target.x = 280.0;
            target.y = 200.0;
            target.hspeed = 0.0;
        }

        let initial_hp = vm
            .current_room
            .instances
            .iter()
            .find(|i| i.id == 2)
            .unwrap()
            .vars
            .get("hp")
            .unwrap()
            .as_f32();

        // Fire Thunderbolt (BTN_Z)
        let mut input = InputSnapshot::default();
        input.pressed = BTN_Z;
        vm.step(&input, &mut audio);

        // Verify attack instance was spawned (at x = 232, target is at 280)
        let attack_exists = vm
            .current_room
            .instances
            .iter()
            .any(|i| i.object_name == "obj_attack_thunderbolt");
        assert!(attack_exists, "Thunderbolt projectile must exist");

        // Step ticks until projectile (hspeed = 8.5) collides with target at x = 280
        let empty_input = InputSnapshot::default();
        for _ in 0..10 {
            vm.step(&empty_input, &mut audio);
        }

        let target_hp = vm
            .current_room
            .instances
            .iter()
            .find(|i| i.id == 2)
            .unwrap()
            .vars
            .get("hp")
            .unwrap()
            .as_f32();

        assert!(target_hp < initial_hp, "Target HP should be reduced on hit");
    }

    #[test]
    fn test_emit_quads_to_renderer() {
        let vm = GmlVm::new_interactive_room(800.0, 480.0);
        let mut renderer = Gles3BatchRenderer::new(800, 480);

        vm.emit_quads(&mut renderer);
        assert!(!renderer.quads_queue.is_empty(), "Quads queue must not be empty");

        let batches = renderer.flush_batches();
        assert!(!batches.is_empty(), "Batches must be generated");

        // Batches must contain both atlas 0 (tiles) and atlas 1 (instances)
        let has_atlas_0 = batches.iter().any(|b| b.atlas_id == 0);
        let has_atlas_1 = batches.iter().any(|b| b.atlas_id == 1);
        assert!(has_atlas_0, "Background tiles batch on atlas 0 must exist");
        assert!(has_atlas_1, "Entity batch on atlas 1 must exist");
    }
}
