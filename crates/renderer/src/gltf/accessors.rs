use awsm_renderer_core::pipeline::vertex::VertexFormat;

pub(super) fn semantic_ordering(semantic: &gltf::Semantic) -> u8 {
    match semantic {
        gltf::Semantic::Positions => 1,
        gltf::Semantic::Normals => 2,
        gltf::Semantic::Tangents => 3,
        gltf::Semantic::Colors(_) => 4,
        gltf::Semantic::TexCoords(_) => 5,
        gltf::Semantic::Joints(_) => 6,
        gltf::Semantic::Weights(_) => 7,
    }
}

pub(super) fn accessor_vertex_format(accessor: &gltf::Accessor<'_>) -> VertexFormat {
    // https://gpuweb.github.io/gpuweb/#enumdef-gpuvertexformat
    match (accessor.data_type(), accessor.dimensions(), accessor.normalized()) {
        // I8: normalized → signed normalized formats; not normalized → signed integer formats.
        (gltf::accessor::DataType::I8, gltf::accessor::Dimensions::Scalar, true)  => VertexFormat::Snorm8,
        (gltf::accessor::DataType::I8, gltf::accessor::Dimensions::Scalar, false) => VertexFormat::Sint8,
        (gltf::accessor::DataType::I8, gltf::accessor::Dimensions::Vec2, true)    => VertexFormat::Snorm8x2,
        (gltf::accessor::DataType::I8, gltf::accessor::Dimensions::Vec2, false)   => VertexFormat::Sint8x2,
        // Vec3 is not directly supported; pad to 4.
        (gltf::accessor::DataType::I8, gltf::accessor::Dimensions::Vec3, true)    => VertexFormat::Snorm8x4,
        (gltf::accessor::DataType::I8, gltf::accessor::Dimensions::Vec3, false)   => VertexFormat::Sint8x4,
        (gltf::accessor::DataType::I8, gltf::accessor::Dimensions::Vec4, true)    => VertexFormat::Snorm8x4,
        (gltf::accessor::DataType::I8, gltf::accessor::Dimensions::Vec4, false)   => VertexFormat::Sint8x4,
        // For matrices, treat as the corresponding vector type (i.e. a Mat2 becomes a Vec2, etc.)
        (gltf::accessor::DataType::I8, gltf::accessor::Dimensions::Mat2, true)    => VertexFormat::Snorm8x2,
        (gltf::accessor::DataType::I8, gltf::accessor::Dimensions::Mat2, false)   => VertexFormat::Sint8x2,
        // Mat3 has 3 columns; pad each column to 4 components.
        (gltf::accessor::DataType::I8, gltf::accessor::Dimensions::Mat3, true)    => VertexFormat::Snorm8x4,
        (gltf::accessor::DataType::I8, gltf::accessor::Dimensions::Mat3, false)   => VertexFormat::Sint8x4,
        (gltf::accessor::DataType::I8, gltf::accessor::Dimensions::Mat4, true)    => VertexFormat::Snorm8x4,
        (gltf::accessor::DataType::I8, gltf::accessor::Dimensions::Mat4, false)   => VertexFormat::Sint8x4,

        // U8: normalized → unsigned normalized formats; not normalized → unsigned integer formats.
        (gltf::accessor::DataType::U8, gltf::accessor::Dimensions::Scalar, true)  => VertexFormat::Unorm8,
        (gltf::accessor::DataType::U8, gltf::accessor::Dimensions::Scalar, false) => VertexFormat::Uint8,
        (gltf::accessor::DataType::U8, gltf::accessor::Dimensions::Vec2, true)    => VertexFormat::Unorm8x2,
        (gltf::accessor::DataType::U8, gltf::accessor::Dimensions::Vec2, false)   => VertexFormat::Uint8x2,
        // Vec3: pad to 4.
        (gltf::accessor::DataType::U8, gltf::accessor::Dimensions::Vec3, true)    => VertexFormat::Unorm8x4,
        (gltf::accessor::DataType::U8, gltf::accessor::Dimensions::Vec3, false)   => VertexFormat::Uint8x4,
        (gltf::accessor::DataType::U8, gltf::accessor::Dimensions::Vec4, true)    => VertexFormat::Unorm8x4,
        (gltf::accessor::DataType::U8, gltf::accessor::Dimensions::Vec4, false)   => VertexFormat::Uint8x4,
        (gltf::accessor::DataType::U8, gltf::accessor::Dimensions::Mat2, true)    => VertexFormat::Unorm8x2,
        (gltf::accessor::DataType::U8, gltf::accessor::Dimensions::Mat2, false)   => VertexFormat::Uint8x2,
        // Mat3: pad to 4.
        (gltf::accessor::DataType::U8, gltf::accessor::Dimensions::Mat3, true)    => VertexFormat::Unorm8x4,
        (gltf::accessor::DataType::U8, gltf::accessor::Dimensions::Mat3, false)   => VertexFormat::Uint8x4,
        (gltf::accessor::DataType::U8, gltf::accessor::Dimensions::Mat4, true)    => VertexFormat::Unorm8x4,
        (gltf::accessor::DataType::U8, gltf::accessor::Dimensions::Mat4, false)   => VertexFormat::Uint8x4,

        // I16: normalized → signed normalized; not normalized → signed integer.
        (gltf::accessor::DataType::I16, gltf::accessor::Dimensions::Scalar, true)  => VertexFormat::Snorm16,
        (gltf::accessor::DataType::I16, gltf::accessor::Dimensions::Scalar, false) => VertexFormat::Sint16,
        (gltf::accessor::DataType::I16, gltf::accessor::Dimensions::Vec2, true)    => VertexFormat::Snorm16x2,
        (gltf::accessor::DataType::I16, gltf::accessor::Dimensions::Vec2, false)   => VertexFormat::Sint16x2,
        // Vec3: pad to 4.
        (gltf::accessor::DataType::I16, gltf::accessor::Dimensions::Vec3, true)    => VertexFormat::Snorm16x4,
        (gltf::accessor::DataType::I16, gltf::accessor::Dimensions::Vec3, false)   => VertexFormat::Sint16x4,
        (gltf::accessor::DataType::I16, gltf::accessor::Dimensions::Vec4, true)    => VertexFormat::Snorm16x4,
        (gltf::accessor::DataType::I16, gltf::accessor::Dimensions::Vec4, false)   => VertexFormat::Sint16x4,
        (gltf::accessor::DataType::I16, gltf::accessor::Dimensions::Mat2, true)    => VertexFormat::Snorm16x2,
        (gltf::accessor::DataType::I16, gltf::accessor::Dimensions::Mat2, false)   => VertexFormat::Sint16x2,
        // Mat3: pad to 4.
        (gltf::accessor::DataType::I16, gltf::accessor::Dimensions::Mat3, true)    => VertexFormat::Snorm16x4,
        (gltf::accessor::DataType::I16, gltf::accessor::Dimensions::Mat3, false)   => VertexFormat::Sint16x4,
        (gltf::accessor::DataType::I16, gltf::accessor::Dimensions::Mat4, true)    => VertexFormat::Snorm16x4,
        (gltf::accessor::DataType::I16, gltf::accessor::Dimensions::Mat4, false)   => VertexFormat::Sint16x4,

        // U16: normalized → unsigned normalized; not normalized → unsigned integer.
        (gltf::accessor::DataType::U16, gltf::accessor::Dimensions::Scalar, true)  => VertexFormat::Unorm16,
        (gltf::accessor::DataType::U16, gltf::accessor::Dimensions::Scalar, false) => VertexFormat::Uint16,
        (gltf::accessor::DataType::U16, gltf::accessor::Dimensions::Vec2, true)    => VertexFormat::Unorm16x2,
        (gltf::accessor::DataType::U16, gltf::accessor::Dimensions::Vec2, false)   => VertexFormat::Uint16x2,
        // Vec3: pad to 4.
        (gltf::accessor::DataType::U16, gltf::accessor::Dimensions::Vec3, true)    => VertexFormat::Unorm16x4,
        (gltf::accessor::DataType::U16, gltf::accessor::Dimensions::Vec3, false)   => VertexFormat::Uint16x4,
        (gltf::accessor::DataType::U16, gltf::accessor::Dimensions::Vec4, true)    => VertexFormat::Unorm16x4,
        (gltf::accessor::DataType::U16, gltf::accessor::Dimensions::Vec4, false)   => VertexFormat::Uint16x4,
        (gltf::accessor::DataType::U16, gltf::accessor::Dimensions::Mat2, true)    => VertexFormat::Unorm16x2,
        (gltf::accessor::DataType::U16, gltf::accessor::Dimensions::Mat2, false)   => VertexFormat::Uint16x2,
        // Mat3: pad to 4.
        (gltf::accessor::DataType::U16, gltf::accessor::Dimensions::Mat3, true)    => VertexFormat::Unorm16x4,
        (gltf::accessor::DataType::U16, gltf::accessor::Dimensions::Mat3, false)   => VertexFormat::Uint16x4,
        (gltf::accessor::DataType::U16, gltf::accessor::Dimensions::Mat4, true)    => VertexFormat::Unorm16x4,
        (gltf::accessor::DataType::U16, gltf::accessor::Dimensions::Mat4, false)   => VertexFormat::Uint16x4,

        // U32: normalized flag is ignored.
        (gltf::accessor::DataType::U32, gltf::accessor::Dimensions::Scalar, _)
            => VertexFormat::Uint32,
        (gltf::accessor::DataType::U32, gltf::accessor::Dimensions::Vec2, _)
            => VertexFormat::Uint32x2,
        (gltf::accessor::DataType::U32, gltf::accessor::Dimensions::Vec3, _)
            => VertexFormat::Uint32x3,
        (gltf::accessor::DataType::U32, gltf::accessor::Dimensions::Vec4, _)
            => VertexFormat::Uint32x4,
        (gltf::accessor::DataType::U32, gltf::accessor::Dimensions::Mat2, _)
            => VertexFormat::Uint32x2,
        (gltf::accessor::DataType::U32, gltf::accessor::Dimensions::Mat3, _)
            => VertexFormat::Uint32x3,
        (gltf::accessor::DataType::U32, gltf::accessor::Dimensions::Mat4, _)
            => VertexFormat::Uint32x4,

        // F32: normalized flag is ignored.
        (gltf::accessor::DataType::F32, gltf::accessor::Dimensions::Scalar, _)
            => VertexFormat::Float32,
        (gltf::accessor::DataType::F32, gltf::accessor::Dimensions::Vec2, _)
            => VertexFormat::Float32x2,
        (gltf::accessor::DataType::F32, gltf::accessor::Dimensions::Vec3, _)
            => VertexFormat::Float32x3,
        (gltf::accessor::DataType::F32, gltf::accessor::Dimensions::Vec4, _)
            => VertexFormat::Float32x4,
        (gltf::accessor::DataType::F32, gltf::accessor::Dimensions::Mat2, _)
            => VertexFormat::Float32x2,
        (gltf::accessor::DataType::F32, gltf::accessor::Dimensions::Mat3, _)
            => VertexFormat::Float32x3,
        (gltf::accessor::DataType::F32, gltf::accessor::Dimensions::Mat4, _)
            => VertexFormat::Float32x4,
    }
}