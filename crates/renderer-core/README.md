# Renderer core

This is a core crate for the awsm renderer.

At this level, it's just a thin wrapper around the WebGPU API. It is intended to be used as a low-level primitive, without the headache of dealing with the raw `web-sys` bindings directly.

The overall approach is to allow native web-sys types throughout the main methods, but have Rust-friendly data types that can be used to create all the descriptors, pipelines, etc. These Rust-friendly data types `impl Into<web_sys::...>` and so they can be passed to the web-sys methods. This allows for a more idiomatic Rust API for all the heavy lifting, while still allowing for the raw web-sys types to be used when needed. 

In some cases like the command encoder, the custom type holds an inner raw web-sys type, and impls Deref to it, so you get a mixture of the original methods and nicer new ones as they are added.

Example usage:


```rust
use awsm_renderer_core::{command::{render_pass::{ColorAttachment, RenderPassDescriptor}, LoadOp, StoreOp}, error::Result, pipeline::{fragment::{ColorTargetState, FragmentState}, vertex::VertexState, PipelineDescriptor}, renderer::{AwsmRenderer, AwsmRendererBuilder}, shaders::ShaderCode};
use web_sys::HtmlCanvasElement;

pub async fn example(canvas: HtmlCanvasElement) -> Result<()> {
    let renderer = AwsmRendererBuilder::new(web_sys::window().unwrap().navigator().gpu())
        .init_adapter().await?
        .init_device().await?
        .init_context(canvas)?
        .build()?;

    static INIT_SHADER_CODE:&'static str = r#"
        @vertex fn vs(@builtin(vertex_index) vertexIndex : u32) -> @builtin(position) vec4f {
            let pos = array(
                vec2f( 0.0,  0.5),  // top center
                vec2f(-0.5, -0.5),  // bottom left
                vec2f( 0.5, -0.5)   // bottom right
            );
        
            return vec4f(pos[vertexIndex], 0.0, 1.0);
        }
        
        @fragment fn fs() -> @location(0) vec4f {
            return vec4f(1.0, 0.0, 0.0, 1.0);
        }
    "#;

    let shader = renderer.compile_shader(&ShaderCode::new(INIT_SHADER_CODE, None).into());

    let vertex = VertexState::new(&shader, None);
    let fragment = FragmentState::new(&shader, None, vec![ColorTargetState::new(renderer.current_context_format())]);

    let pipeline_descriptor = PipelineDescriptor::new(vertex, None)
        .with_fragment(fragment);


    let pipeline = renderer.create_pipeline(&pipeline_descriptor.into()).await?;


    let command_encoder = renderer.create_command_encoder(None);

    let render_pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
        color_attachments: vec![
            ColorAttachment::new(&renderer.current_context_texture_view()?, LoadOp::Clear, StoreOp::Store)
        ],
        ..Default::default()
    }.into())?;

    render_pass.set_pipeline(&pipeline);
    render_pass.draw(3);
    render_pass.end();

    renderer.submit_commands(&command_encoder.finish());

    Ok(())
}
```

_note: for things to look good, you'll want to make sure you (re)set the canvas size_

One way to do this is to use the [ResizeObserver from awsm-web](https://docs.rs/awsm_web/0.45.0/awsm_web/dom/resize/struct.ResizeObserver.html):


```rust
let resize_observer = ResizeObserver::new(
    move |entries| {
        if let Some(entry) = entries.get(0) {
            let width = entry.content_box_sizes[0].inline_size;
            let height = entry.content_box_sizes[0].block_size;
            canvas.set_width(width);
            canvas.set_height(height);
        }
    }),
    None
);

resize_observer.observe(&canvas);
```