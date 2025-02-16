#import bevy_pbr::{
    mesh_view_bindings::globals,
    pbr_fragment::pbr_input_from_standard_material,
    forward_io::{VertexOutput, Vertex, FragmentOutput},
    mesh_functions::{get_world_from_local, mesh_position_local_to_clip, mesh_normal_local_to_world},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}

const NUM_COEFFICIENTS = 9;

@group(2) @binding(100) var<storage, read> coefficients: array<f32, NUM_COEFFICIENTS>;

const pi = radians(180.0);
const EPSILON = 0.001;

struct Orbitals
{
    s : f32,
    p : f32,
    d : f32,
    sum : f32,
}

fn spherical_harmonics(p : vec3<f32>) -> Orbitals {
    let x = p.x;
    let y = p.y;
    let z = p.z;
    let x2 = p.x * p.x;
    let y2 = p.y * p.y;
    let z2 = p.z * p.z;

    let harmonics = array(
        1/(2*sqrt(pi)),
        sqrt(3/(4*pi)) * x,
        sqrt(3/(4*pi)) * y,
        sqrt(3/(4*pi)) * z,
        0.5 * sqrt(15/pi) * x * y,
        0.5 * sqrt(15/pi) * y * z,
        0.25 * sqrt(5/pi) * (3 * z2 - 1),
        0.5 * sqrt(15/pi) * x * z,
        0.25 * sqrt(15/pi) * (x2 - y2),
    );

    var os : f32 = 0.;
    var op : f32 = 0.;
    var od : f32 = 0.;

    // for (var i = 0; i < NUM_COEFFICIENTS; i+= 1)
    // {
    //     sum += harmonics[i] * coefficients[i];
    // }

    os = harmonics[0] * coefficients[0];

    op = harmonics[1] * coefficients[1] + harmonics[2] * coefficients[2] + harmonics[3] * coefficients[3];

    for (var i = 4; i <=9; i += 1)
    {
        od += harmonics[i] * coefficients[i];
    }

    

    var orbitals : Orbitals;

    orbitals.s = os;
    orbitals.p = op;
    orbitals.d = od;

    orbitals.sum = orbitals.s + orbitals.p + orbitals.d;

    return orbitals;

    //return sum;
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    let p = vertex.position;

    let tangent = vertex.tangent.xyz;

    let bitangent: vec3<f32> = cross(vertex.normal, tangent);

    let offset_t = p + tangent * EPSILON;
    let offset_b = p + bitangent * EPSILON;

    let orbitals_p = spherical_harmonics(p);
    let orbitals_t = spherical_harmonics(offset_t);
    let orbitals_b = spherical_harmonics(offset_b);

    let disp_p = orbitals_p.sum * p;
    let disp_t = orbitals_t.sum * offset_t - disp_p;
    let disp_b = orbitals_b.sum * offset_b - disp_p;

    let normal = cross(disp_t, disp_b);

    var out: VertexOutput;
    out.position = mesh_position_local_to_clip(
        get_world_from_local(vertex.instance_index),
        vec4<f32>(disp_p, 1.0),
    );
    out.world_normal = mesh_normal_local_to_world(normal, vertex.instance_index) * sign(orbitals_p.sum);
    out.color = vec4<f32>(orbitals_p.s, orbitals_p.p, orbitals_p.d, 1.0);
    return out;
}


@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front : bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    var out: FragmentOutput;

    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}