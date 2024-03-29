/*
 * there are some shaders which are compiled and cached right at the start
 * but the workhorse "mesh" shader is build by passing in
 * a MeshVertexShader and MeshFragmentShader struct
 * which are used to first try and load the resulting program from cache
 * which in turn tries to load the individual shaders from cache
 * and if any of these don't exist, compiles ad-hoc
 * making replacemenets to the "uber-shader" as per the struct data
 */
use std::collections::hash_map::Entry;
use crate::prelude::*; 
use awsm_web::webgl::{Id, WebGl2Renderer, ShaderType};
use beach_map::{BeachMap, DefaultVersion};
use rustc_hash::{FxHashMap, FxHashSet};

mod fragment;
pub use fragment::*;
mod vertex;
pub use vertex::*;

pub(super) const COMMON_CAMERA:&'static str = include_str!("./shaders/glsl/common/camera.glsl");
pub(super) const COMMON_MATH:&'static str = include_str!("./shaders/glsl/common/math.glsl");
pub(super) const COMMON_COLOR_SPACE:&'static str = include_str!("./shaders/glsl/common/color_space.glsl");

pub struct ShaderCache {
    pub(crate) programs: ProgramCache,
    pub(crate) vertices: VertexCache,
    pub(crate) fragments: FragmentCache,
}

type MaxLights = u32;

pub(crate) struct ProgramCache {
    pub sprite: Id,
    pub panorama_cubemap: Id,
    pub skybox: Id,
    pub mesh: FxHashMap<(ShaderKey, MaxLights), Id>,
}

// merely a key to hash ad-hoc shader generation
// is not stored on the mesh itself
//
// uniform and other runtime data for mesh
// is controlled via various components as-needed
#[derive(Hash, Debug, Clone, PartialEq, Eq, Default)]
pub struct ShaderKey {
    pub position_attribute_loc: Option<u32>,
    pub normal_attribute_loc: Option<u32>,
    pub tangent_attribute_loc: Option<u32>,
    pub morph_targets: Vec<MorphTarget>,
    pub skin_targets: Vec<SkinTarget>,
    pub n_morph_target_weights: u8,
    pub n_skin_joints: u8,
    pub tex_coords: Option<Vec<u32>>,
    pub vertex_colors: Option<Vec<VertexColor>>,
    pub normal_texture_uv_index: Option<u32>,
    pub metallic_roughness_texture_uv_index: Option<u32>,
    pub base_color_texture_uv_index: Option<u32>,
    pub emissive_texture_uv_index: Option<u32>,
    pub alpha_mode: ShaderKeyAlphaMode,
}

#[derive(Hash, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderKeyAlphaMode {
    Opaque,
    Blend,
    Mask
}

impl Default for ShaderKeyAlphaMode {
    fn default() -> Self {
        Self::Opaque
    }
}

impl AwsmRenderer {
    pub fn mesh_program(&mut self, key: ShaderKey, max_lights: u32) -> Result<Id> {
        let shaders = &mut self.shaders;
        let gl = &mut self.gl;

        match shaders.programs.mesh.entry((key.clone(), max_lights)) {
            Entry::Occupied(entry) => Ok(entry.get().clone()),
            Entry::Vacant(entry) => {

                let vertex_id = shaders.vertices.mesh_shader(gl, &key)?;
                let fragment_id = shaders.fragments.mesh_shader(gl, &key, self.lights.max_lights)?;
                let program_id = gl.compile_program(&vec![vertex_id, fragment_id])?;

                // need to do for each ubo
                gl.init_uniform_buffer_name(program_id, "ubo_camera")?;
                if max_lights > 0 {
                    gl.init_uniform_buffer_name(program_id, "ubo_lights")?;
                }

                Ok(entry.insert(program_id).clone())
            }
        }
    }

    pub fn recompile_mesh_programs_max_lights(&mut self, world: &World, max_lights: u32) -> Result<()> {
        // only recompile existing meshes. 
        // New ones will inherently need to have their program id available
        world.run(|mut meshes: ViewMut<Mesh>| -> Result<()> {
            let mut n_updated = 0;

            for mesh in (&mut meshes).iter() {
                mesh.program_id = self.mesh_program(mesh.shader_key.clone(), max_lights)?;
                n_updated += 1;
            }

            if n_updated > 0 {
                log::warn!("recompiled {n_updated} mesh shaders");
            }
            Ok(())
        })?;

        Ok(())
    }
}

impl ShaderCache {
    pub fn new(mut gl:&mut WebGl2Renderer) -> Result<Self> {
        let vertices = VertexCache::new(&mut gl)?;
        let fragments = FragmentCache::new(&mut gl)?;
        let programs = ProgramCache::new(&mut gl, &vertices, &fragments)?;
        Ok(Self {
            programs,
            vertices,
            fragments
        })
    }
}
impl ProgramCache { 
    pub fn new(mut gl:&mut WebGl2Renderer, vertex_ids: &VertexCache, fragment_ids: &FragmentCache) -> Result<Self> {
        let _self = Self {
            sprite: gl.compile_program(&vec![vertex_ids.quad_unit, fragment_ids.unlit_diffuse])?,
            panorama_cubemap: gl.compile_program(&vec![vertex_ids.fullscreen_triangle, fragment_ids.panorama_to_cubemap])?,
            skybox: gl.compile_program(&vec![vertex_ids.skybox, fragment_ids.skybox])?,
            mesh: FxHashMap::default(),
        };

        for program_id in vec![ 
            _self.sprite, 
        ] {
            // need to do for each ubo
            gl.init_uniform_buffer_name(program_id, "ubo_camera")?;
        }

        Ok(_self)
    }

}
