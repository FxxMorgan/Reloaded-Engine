/// GLSL ES 3.00 vertex shader for batch rendering on TBDR GPUs.
pub const VERTEX_SHADER_ES30: &str = r#"#version 300 es
precision highp float;

layout(location = 0) in vec2 a_position;
layout(location = 1) in vec2 a_texcoord;
layout(location = 2) in vec4 a_color;

layout(std140) uniform FrameUniforms {
    mat4 u_projection;
    vec4 u_view_offset;
    vec4 u_tint_color;
    vec4 u_render_params;
};

out vec2 v_texcoord;
out vec4 v_color;

void main() {
    vec2 pos = a_position + u_view_offset.xy;
    gl_Position = u_projection * vec4(pos, 0.0, 1.0);
    v_texcoord = a_texcoord;
    v_color = a_color * u_tint_color;
}
"#;

/// GLSL ES 3.00 fragment shader for batch rendering on TBDR GPUs.
pub const FRAGMENT_SHADER_ES30: &str = r#"#version 300 es
precision mediump float;

in vec2 v_texcoord;
in vec4 v_color;

uniform sampler2D u_texture;

out vec4 fragColor;

void main() {
    fragColor = texture(u_texture, v_texcoord) * v_color;
}
"#;

/// Uniform buffer block strictly conforming to OpenGL ES std140 layout.
/// Total size MUST be a multiple of 16 bytes (112 bytes total) and 16-byte aligned
/// to satisfy Mali, PowerVR, and Adreno TBDR hardware requirements.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Std140UniformBlock {
    pub projection: [[f32; 4]; 4], // 64 bytes
    pub view_offset: [f32; 4],     // 16 bytes
    pub tint_color: [f32; 4],      // 16 bytes
    pub render_params: [f32; 4],   // 16 bytes
}

impl Std140UniformBlock {
    /// Creates an orthographic 2D projection uniform block for the given viewport.
    pub fn new_ortho(width: f32, height: f32) -> Self {
        let (w, h) = if width <= 0.0 || height <= 0.0 {
            (800.0, 480.0)
        } else {
            (width, height)
        };

        // Orthographic projection mapping [0, w] -> [-1, 1] and [0, h] -> [1, -1] (top-left 0,0)
        let projection = [
            [2.0 / w, 0.0, 0.0, 0.0],
            [0.0, -2.0 / h, 0.0, 0.0],
            [0.0, 0.0, -1.0, 0.0],
            [-1.0, 1.0, 0.0, 1.0],
        ];

        Self {
            projection,
            view_offset: [0.0, 0.0, 0.0, 0.0],
            tint_color: [1.0, 1.0, 1.0, 1.0],
            render_params: [w, h, 0.0, 0.0],
        }
    }
}

/// Single 2D textured vertex with RGBA color.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpriteVertex {
    pub position: [f32; 2],
    pub tex_coord: [f32; 2],
    pub color: [f32; 4],
}

/// High-level 2D sprite quad to be batched.
#[derive(Debug, Clone, PartialEq)]
pub struct SpriteQuad {
    pub atlas_id: u32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
    pub color: [f32; 4],
}

impl SpriteQuad {
    pub fn new(
        atlas_id: u32,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        u0: f32,
        v0: f32,
        u1: f32,
        v1: f32,
        color: [f32; 4],
    ) -> Self {
        Self {
            atlas_id,
            x,
            y,
            width,
            height,
            u0,
            v0,
            u1,
            v1,
            color,
        }
    }

    /// Decomposes this quad into 6 vertices forming 2 triangles.
    pub fn to_vertices(&self) -> [SpriteVertex; 6] {
        let x0 = self.x;
        let y0 = self.y;
        let x1 = self.x + self.width;
        let y1 = self.y + self.height;

        let tl = SpriteVertex {
            position: [x0, y0],
            tex_coord: [self.u0, self.v0],
            color: self.color,
        };
        let tr = SpriteVertex {
            position: [x1, y0],
            tex_coord: [self.u1, self.v0],
            color: self.color,
        };
        let br = SpriteVertex {
            position: [x1, y1],
            tex_coord: [self.u1, self.v1],
            color: self.color,
        };
        let bl = SpriteVertex {
            position: [x0, y1],
            tex_coord: [self.u0, self.v1],
            color: self.color,
        };

        // Triangles: (tl, tr, br) and (tl, br, bl)
        [tl, tr, br, tl, br, bl]
    }
}

/// A contiguous slice of vertices sharing the same texture atlas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawBatch {
    pub atlas_id: u32,
    pub vertex_offset: usize,
    pub vertex_count: usize,
    pub quad_count: usize,
}

/// State tracker ensuring minimal redundant GL state switches on mobile GPUs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GlStateTracker {
    pub bound_program: u32,
    pub bound_vao: u32,
    pub bound_vbo: u32,
    pub bound_atlas_id: u32,
}

/// RAM-cached atlas texture page that survives Android surface destruction.
#[derive(Debug, Clone, PartialEq)]
pub struct RamAtlasPage {
    pub id: u32,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
    pub is_gpu_uploaded: bool,
    pub gpu_texture_id: u32,
}

/// Batch renderer tailored for Android TBDR architectures (PowerVR, Mali, Adreno).
/// Keeps full asset state in RAM while tracking GL state and handling surface lifecycles safely.
#[derive(Debug)]
pub struct Gles3BatchRenderer {
    pub surface_width: u32,
    pub surface_height: u32,
    pub surface_active: bool,
    pub gpu_resources_valid: bool,

    pub uniforms: Std140UniformBlock,
    pub state_tracker: GlStateTracker,

    pub quads_queue: Vec<SpriteQuad>,
    pub vertex_buffer: Vec<SpriteVertex>,
    pub batches: Vec<DrawBatch>,

    pub ram_atlases: Vec<RamAtlasPage>,
    pub total_draw_calls: u64,
}

impl Default for Gles3BatchRenderer {
    fn default() -> Self {
        Self::new(800, 480)
    }
}

impl Gles3BatchRenderer {
    /// Creates a new `Gles3BatchRenderer`.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            surface_width: width,
            surface_height: height,
            surface_active: false,
            gpu_resources_valid: false,
            uniforms: Std140UniformBlock::new_ortho(width as f32, height as f32),
            state_tracker: GlStateTracker::default(),
            quads_queue: Vec::with_capacity(1024),
            vertex_buffer: Vec::with_capacity(6144),
            batches: Vec::with_capacity(64),
            ram_atlases: Vec::new(),
            total_draw_calls: 0,
        }
    }

    /// Registers a RAM-resident atlas page. RAM data persists even if the surface is destroyed.
    pub fn register_ram_atlas(&mut self, id: u32, width: u32, height: u32, data: Vec<u8>) {
        if let Some(existing) = self.ram_atlases.iter_mut().find(|a| a.id == id) {
            existing.width = width;
            existing.height = height;
            existing.data = data;
            existing.is_gpu_uploaded = false;
        } else {
            self.ram_atlases.push(RamAtlasPage {
                id,
                width,
                height,
                data,
                is_gpu_uploaded: false,
                gpu_texture_id: 0,
            });
        }
    }

    /// Submits a sprite quad to the draw queue.
    pub fn submit_quad(&mut self, quad: SpriteQuad) {
        self.quads_queue.push(quad);
    }

    /// Sorts and groups queued quads by `atlas_id` into `DrawBatch` instances.
    /// This maximizes batching and minimizes state switches on TBDR mobile GPUs.
    pub fn flush_batches(&mut self) -> &[DrawBatch] {
        self.vertex_buffer.clear();
        self.batches.clear();

        if self.quads_queue.is_empty() {
            return &self.batches;
        }

        // Sort quads by atlas_id to avoid switching textures back and forth
        self.quads_queue.sort_by_key(|q| q.atlas_id);

        let mut current_atlas = self.quads_queue[0].atlas_id;
        let mut batch_start_vert = 0;
        let mut quad_count = 0;

        for quad in &self.quads_queue {
            if quad.atlas_id != current_atlas {
                // End current batch
                let vert_count = self.vertex_buffer.len() - batch_start_vert;
                self.batches.push(DrawBatch {
                    atlas_id: current_atlas,
                    vertex_offset: batch_start_vert,
                    vertex_count: vert_count,
                    quad_count,
                });

                // Start new batch
                current_atlas = quad.atlas_id;
                batch_start_vert = self.vertex_buffer.len();
                quad_count = 0;
            }

            let verts = quad.to_vertices();
            self.vertex_buffer.extend_from_slice(&verts);
            quad_count += 1;
        }

        // Push final batch
        let vert_count = self.vertex_buffer.len() - batch_start_vert;
        if vert_count > 0 {
            self.batches.push(DrawBatch {
                atlas_id: current_atlas,
                vertex_offset: batch_start_vert,
                vertex_count: vert_count,
                quad_count,
            });
        }

        self.quads_queue.clear();
        &self.batches
    }

    /// Dispatches state tracking changes on batch drawing.
    /// Returns the number of draw calls dispatched.
    pub fn execute_render_pass(&mut self) -> u32 {
        if !self.surface_active || !self.gpu_resources_valid {
            return 0;
        }

        if !self.quads_queue.is_empty() {
            self.flush_batches();
        }

        let mut draw_calls = 0;

        for batch in &self.batches {
            if self.state_tracker.bound_atlas_id != batch.atlas_id {
                self.state_tracker.bound_atlas_id = batch.atlas_id;
                // Atlas texture switch recorded
            }
            draw_calls += 1;
            self.total_draw_calls += 1;
        }

        draw_calls
    }

    /// Android surface created: initializes projection and marks GPU resources ready.
    pub fn on_surface_created(&mut self, width: u32, height: u32) {
        self.surface_width = width;
        self.surface_height = height;
        self.surface_active = true;
        self.gpu_resources_valid = true;
        self.state_tracker = GlStateTracker::default();
        self.uniforms = Std140UniformBlock::new_ortho(width as f32, height as f32);

        // Note: RAM atlases remain in RAM and will be re-uploaded on next render
        for atlas in &mut self.ram_atlases {
            atlas.is_gpu_uploaded = false;
            atlas.gpu_texture_id = 0;
        }
    }

    /// Android surface changed (e.g. orientation or resolution change).
    pub fn on_surface_changed(&mut self, width: u32, height: u32) {
        self.surface_width = width;
        self.surface_height = height;
        self.uniforms = Std140UniformBlock::new_ortho(width as f32, height as f32);
    }

    /// Android surface destroyed (app moved to background).
    /// Strictly avoids `glInvalidateFramebuffer`.
    /// Marks GPU texture handles as lost while PRESERVING all RAM atlas data,
    /// preventing black screens or crashes upon returning!
    pub fn on_surface_destroyed(&mut self) {
        self.surface_active = false;
        self.gpu_resources_valid = false;
        self.state_tracker = GlStateTracker::default();

        // Mark textures as dirty for recreation, but KEEP raw byte buffers in RAM!
        for atlas in &mut self.ram_atlases {
            atlas.is_gpu_uploaded = false;
            atlas.gpu_texture_id = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_std140_uniform_block_size_and_alignment() {
        let size = std::mem::size_of::<Std140UniformBlock>();
        let align = std::mem::align_of::<Std140UniformBlock>();

        // Exact 112 bytes and multiple of 16 for std140 layout
        assert_eq!(size, 112, "Std140UniformBlock must be exactly 112 bytes");
        assert_eq!(size % 16, 0, "Std140UniformBlock size must be a multiple of 16");
        assert_eq!(align, 16, "Std140UniformBlock must be 16-byte aligned");

        let uniforms = Std140UniformBlock::new_ortho(800.0, 480.0);
        assert_eq!(uniforms.tint_color, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(uniforms.render_params[0], 800.0);
        assert_eq!(uniforms.render_params[1], 480.0);
    }

    #[test]
    fn test_sprite_batching_by_atlas_id() {
        let mut renderer = Gles3BatchRenderer::new(800, 480);
        renderer.on_surface_created(800, 480);

        // Submit quads with interleaved atlas IDs (0, 1, 0, 2, 1, 0)
        renderer.submit_quad(SpriteQuad::new(0, 0.0, 0.0, 32.0, 32.0, 0.0, 0.0, 1.0, 1.0, [1.0; 4]));
        renderer.submit_quad(SpriteQuad::new(1, 10.0, 10.0, 32.0, 32.0, 0.0, 0.0, 1.0, 1.0, [1.0; 4]));
        renderer.submit_quad(SpriteQuad::new(0, 20.0, 20.0, 32.0, 32.0, 0.0, 0.0, 1.0, 1.0, [1.0; 4]));
        renderer.submit_quad(SpriteQuad::new(2, 30.0, 30.0, 32.0, 32.0, 0.0, 0.0, 1.0, 1.0, [1.0; 4]));
        renderer.submit_quad(SpriteQuad::new(1, 40.0, 40.0, 32.0, 32.0, 0.0, 0.0, 1.0, 1.0, [1.0; 4]));
        renderer.submit_quad(SpriteQuad::new(0, 50.0, 50.0, 32.0, 32.0, 0.0, 0.0, 1.0, 1.0, [1.0; 4]));

        let batches = renderer.flush_batches();
        // Sorting by atlas_id should merge them into exactly 3 batches (atlas 0: 3 quads, atlas 1: 2 quads, atlas 2: 1 quad)
        assert_eq!(batches.len(), 3);
        assert_eq!(batches[0].atlas_id, 0);
        assert_eq!(batches[0].quad_count, 3);
        assert_eq!(batches[0].vertex_count, 18);

        assert_eq!(batches[1].atlas_id, 1);
        assert_eq!(batches[1].quad_count, 2);
        assert_eq!(batches[1].vertex_count, 12);

        assert_eq!(batches[2].atlas_id, 2);
        assert_eq!(batches[2].quad_count, 1);
        assert_eq!(batches[2].vertex_count, 6);

        let calls = renderer.execute_render_pass();
        assert_eq!(calls, 3);
    }

    #[test]
    fn test_surface_lifecycle_preserves_ram_state() {
        let mut renderer = Gles3BatchRenderer::new(800, 480);
        let sample_pixels = vec![255u8; 64 * 64 * 4];
        renderer.register_ram_atlas(0, 64, 64, sample_pixels.clone());

        renderer.on_surface_created(800, 480);
        assert!(renderer.surface_active);
        assert!(renderer.gpu_resources_valid);

        // Surface destroyed (backgrounded)
        renderer.on_surface_destroyed();
        assert!(!renderer.surface_active);
        assert!(!renderer.gpu_resources_valid);

        // RAM atlas data must still be intact!
        assert_eq!(renderer.ram_atlases.len(), 1);
        assert_eq!(renderer.ram_atlases[0].data, sample_pixels);
        assert!(!renderer.ram_atlases[0].is_gpu_uploaded);

        // Surface re-created (resumed)
        renderer.on_surface_created(1920, 1080);
        assert!(renderer.surface_active);
        assert!(renderer.gpu_resources_valid);
        assert_eq!(renderer.surface_width, 1920);
        assert_eq!(renderer.surface_height, 1080);
        assert_eq!(renderer.ram_atlases[0].data, sample_pixels);
    }
}
