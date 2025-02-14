#import bevy_pbr::{
    mesh_view_bindings::globals,
}

#import bevy_pbr::mesh_functions::{get_world_from_local, mesh_position_local_to_clip}

#import bevy_pbr::forward_io::{VertexOutput, Vertex}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    out.position = mesh_position_local_to_clip(
        get_world_from_local(vertex.instance_index),
        vec4<f32>(vertex.position+0.5*vertex.normal, 1.0),
    );
    return out;
}


@fragment
fn fragment(vertex_output: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}