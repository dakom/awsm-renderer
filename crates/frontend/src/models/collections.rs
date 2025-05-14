use std::collections::HashMap;
use std::hash::Hash;

use crate::prelude::*;

pub static GLTF_SETS: LazyLock<HashMap<&'static str, Vec<GltfId>>> = LazyLock::new(|| {
    let mut h = HashMap::new();

    h.insert("Todo", vec![GltfId::AlphaBlendMode]);

    // h.insert(
    //     "Feature tests",
    //     vec![
    //         GltfId::AlphaBlendMode,
    //         GltfId::BoomBoxAxes,
    //         //GltfId::MetalRoughSpheres,
    //         //GltfId::MetalRoughSpheresTextureless,
    //         GltfId::MorphPrimitives,
    //         //GltfId::MorphStressTest,
    //         GltfId::MultiUv,
    //         //GltfId::NegativeScale,
    //         // GltfId::NormalTangent,
    //         // GltfId::NormalTangentMirror,
    //         GltfId::Orientation,
    //         //GltfId::RecursiveSkeletons,
    //         GltfId::TextureCoordinate,
    //         GltfId::TextureLinearInterpolation,
    //         GltfId::TextureSettings,
    //         GltfId::VertexColor,
    //     ],
    // );

    h.insert(
        "Simple",
        vec![
            GltfId::TriangleWithoutIndices,
            GltfId::Triangle,
            GltfId::SimpleSparseAccessor,
            GltfId::SimpleMeshes,
            GltfId::SimpleTexture,
            GltfId::SimpleInstancing,
            GltfId::SimpleMaterial,
        ],
    );

    h.insert(
        "Animation",
        vec![
            GltfId::SimpleSkin,
            GltfId::SimpleMorph,
            GltfId::AnimatedTriangle,
            GltfId::AnimatedMorphCube,
            GltfId::InterpolationTest,
        ],
    );

    // h.insert(
    //     "Standard",
    //     vec![
    //         GltfId::Box,
    //         GltfId::BoxInterleaved,
    //         GltfId::BoxTextured,
    //         GltfId::BoxTexturedNpoT,
    //         GltfId::BoxWithSpaces,
    //         GltfId::BoxVertexColors,
    //         GltfId::Cube,
    //     ],
    // );

    // h.insert(
    //     "Extension Tests",
    //     vec![
    //         // GltfId::EnvironmentTest,
    //         // GltfId::EnvironmentIblTest,
    //     ],
    // );

    h
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GltfId {
    // FEATURE TESTS
    // https://github.com/KhronosGroup/glTF-Sample-Models/tree/master/2.0#feature-tests
    AlphaBlendMode,
    BoomBoxAxes,
    MetalRoughSpheres,
    MetalRoughSpheresTextureless,
    MorphPrimitives,
    MorphStressTest,
    MultiUv,
    NegativeScale,
    NormalTangent,
    NormalTangentMirror,
    Orientation,
    RecursiveSkeletons,
    TextureCoordinate,
    TextureLinearInterpolation,
    TextureSettings,
    VertexColor,

    // MINIMAL
    // https://github.com/KhronosGroup/glTF-Sample-Models/tree/master/2.0#minimal-tests
    TriangleWithoutIndices,
    Triangle,
    SimpleSparseAccessor,
    SimpleMeshes,
    SimpleMorph,
    AnimatedTriangle,
    AnimatedMorphCube,
    AnimatedMorphSphere,
    SimpleSkin,
    SimpleInstancing,
    SimpleTexture,
    SimpleMaterial,
    InterpolationTest,
    // skipping unicode test...

    // STANDARD
    // https://github.com/KhronosGroup/glTF-Sample-Models/tree/master/2.0#standard
    Box,
    BoxInterleaved,
    BoxTextured,
    BoxTexturedNpoT,
    BoxWithSpaces,
    BoxVertexColors,
    Cube,

    // EXTENSION TESTS
    // https://github.com/KhronosGroup/glTF-Sample-Models/tree/master/2.0#feature-tests-1
    EnvironmentTest,
    EnvironmentIblTest,
}

impl TryFrom<&str> for GltfId {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let list: Vec<&GltfId> = GLTF_SETS.iter().map(|x| x.1).flatten().collect();

        for id in list {
            let id_str = id.to_string();
            if id_str == s {
                return Ok(*id);
            }
        }

        Err(format!("{} is not a valid GltfId", s))
    }
}

impl std::fmt::Display for GltfId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl GltfId {
    pub fn find_set_label(&self) -> &'static str {
        let res = GLTF_SETS.iter().find(|x| x.1.contains(self));
        res.unwrap().0
    }

    pub fn filepath(&self) -> &'static str {
        match self {
            // Feature tests
            Self::AlphaBlendMode => "AlphaBlendModeTest/glTF/AlphaBlendModeTest.gltf",
            Self::BoomBoxAxes => "BoomBoxWithAxes/glTF/BoomBoxWithAxes.gltf",
            Self::MetalRoughSpheres => "MetalRoughSpheres/glTF/MetalRoughSpheres.gltf",
            Self::MetalRoughSpheresTextureless => {
                "MetalRoughSpheresNoTextures/glTF/MetalRoughSpheresNoTextures.gltf"
            }
            Self::MorphPrimitives => "MorphPrimitivesTest/glTF/MorphPrimitivesTest.gltf",
            Self::MorphStressTest => "MorphStressTest/glTF/MorphStressTest.gltf",
            Self::MultiUv => "MultiUVTest/glTF/MultiUVTest.gltf",
            Self::NegativeScale => "NegativeScaleTest/glTF/NegativeScaleTest.gltf",
            Self::NormalTangent => "NormalTangentTest/glTF/NormalTangentTest.gltf",
            Self::NormalTangentMirror => {
                "NormalTangentMirrorTest/glTF/NormalTangentMirrorTest.gltf"
            }
            Self::Orientation => "OrientationTest/glTF/OrientationTest.gltf",
            Self::RecursiveSkeletons => "RecursiveSkeletons/glTF/RecursiveSkeletons.gltf",
            Self::TextureCoordinate => "TextureCoordinateTest/glTF/TextureCoordinateTest.gltf",
            Self::TextureLinearInterpolation => {
                "TextureLinearInterpolationTest/glTF/TextureLinearInterpolationTest.gltf"
            }
            Self::TextureSettings => "TextureSettingsTest/glTF/TextureSettingsTest.gltf",
            Self::VertexColor => "VertexColorTest/glTF/VertexColorTest.gltf",
            // Minimal
            Self::TriangleWithoutIndices => {
                "TriangleWithoutIndices/glTF/TriangleWithoutIndices.gltf"
            }
            Self::Triangle => "Triangle/glTF/Triangle.gltf",
            Self::SimpleSparseAccessor => "SimpleSparseAccessor/glTF/SimpleSparseAccessor.gltf",
            Self::SimpleMeshes => "SimpleMeshes/glTF/SimpleMeshes.gltf",
            Self::SimpleMorph => "SimpleMorph/glTF/SimpleMorph.gltf",
            Self::SimpleInstancing => "SimpleInstancing/glTF/SimpleInstancing.gltf",
            Self::SimpleTexture => "SimpleTexture/glTF/SimpleTexture.gltf",
            Self::SimpleMaterial => "SimpleMaterial/glTF/SimpleMaterial.gltf",
            Self::AnimatedTriangle => "AnimatedTriangle/glTF/AnimatedTriangle.gltf",
            Self::AnimatedMorphCube => "AnimatedMorphCube/glTF/AnimatedMorphCube.gltf",
            Self::AnimatedMorphSphere => "AnimatedMorphSphere/glTF/AnimatedMorphSphere.gltf",
            Self::SimpleSkin => "SimpleSkin/glTF/SimpleSkin.gltf",
            Self::InterpolationTest => "InterpolationTest/glTF/InterpolationTest.gltf",

            // Standard
            Self::Box => "Box/glTF/Box.gltf",
            Self::BoxInterleaved => "BoxInterleaved/glTF/BoxInterleaved.gltf",
            Self::BoxTextured => "BoxTextured/glTF/BoxTextured.gltf",
            Self::BoxTexturedNpoT => "BoxTexturedNonPowerOfTwo/glTF/BoxTexturedNonPowerOfTwo.gltf",
            Self::BoxWithSpaces => "Box With Spaces/glTF/Box With Spaces.gltf",
            Self::BoxVertexColors => "BoxVertexColors/glTF/BoxVertexColors.gltf",
            Self::Cube => "Cube/glTF/Cube.gltf",

            // Extension Tests
            Self::EnvironmentTest => "EnvironmentTest/glTF/EnvironmentTest.gltf",
            Self::EnvironmentIblTest => "EnvironmentTest/glTF-IBL/EnvironmentTest.gltf",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            // feature tests
            Self::AlphaBlendMode => "Alpha blend mode",
            Self::BoomBoxAxes => "Boom box w/ axes",
            Self::MetalRoughSpheres => "Metal rough spheres",
            Self::MetalRoughSpheresTextureless => "Metal rough spheres w/o textures",
            Self::MorphPrimitives => "Morph primitives",
            Self::MorphStressTest => "Morph stress test",
            Self::MultiUv => "Multi uvs",
            Self::NegativeScale => "Negative scale",
            Self::NormalTangent => "Normal tangent auto",
            Self::NormalTangentMirror => "Normal tangent supplied",
            Self::Orientation => "Orientation",
            Self::RecursiveSkeletons => "Recursive skeletons",
            Self::TextureCoordinate => "Texture coordinates",
            Self::TextureLinearInterpolation => "Linear texture interpolation",
            Self::TextureSettings => "Texture settings",
            Self::VertexColor => "Vertex colors",

            // Minimal
            Self::TriangleWithoutIndices => "Triangle without indices",
            Self::Triangle => "Triangle",
            Self::SimpleSparseAccessor => "Simple Sparse Accessor",
            Self::SimpleMeshes => "Simple Meshes",
            Self::SimpleMorph => "Simple Morph",
            Self::SimpleInstancing => "Simple Instancing",
            Self::SimpleTexture => "Simple Texture",
            Self::SimpleMaterial => "Simple Material",
            Self::AnimatedTriangle => "Animated Triangle",
            Self::AnimatedMorphCube => "Animated Morph Cube",
            Self::AnimatedMorphSphere => "Animated Morph Sphere",
            Self::SimpleSkin => "Simple Skin",
            Self::InterpolationTest => "Interpolation Test",

            // Standard
            Self::Box => "Box",
            Self::BoxInterleaved => "BoxInterleaved",
            Self::BoxTextured => "BoxTextured",
            Self::BoxTexturedNpoT => "BoxTextured non-power-of-2",
            Self::BoxWithSpaces => "Box with spaces",
            Self::BoxVertexColors => "Box vertex colors",
            Self::Cube => "Cube",

            // Extension Tests
            Self::EnvironmentTest => "Environment test",
            Self::EnvironmentIblTest => "Environment ibl test",
        }
    }
}
