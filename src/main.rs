use bevy_kira_audio::prelude::*;
use bevy::{
    prelude::*,
    reflect::TypePath,
    render::{
        render_resource::{AsBindGroup, ShaderRef},
        storage::ShaderStorageBuffer,
        mesh::SphereKind
    },
    pbr::{ExtendedMaterial,MaterialExtension},
    core_pipeline::{
        bloom::Bloom,
        dof::{DepthOfField, DepthOfFieldMode},
        tonemapping::Tonemapping,
    },
    prelude::ops::{sin, cos},
    math::FloatOrd,
};
use bevy_common_assets::toml::TomlAssetPlugin;
use serde::{Deserialize};
use std::cmp::max;

const HARMONICS_SHADER_PATH: &str = "shaders/sphericalharmonics.wgsl";
const NUM_COEFFICIENTS: usize = 9;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            AudioPlugin,
            MaterialPlugin::<ExtendedMaterial<StandardMaterial, SphericalHarmonicsMaterial>>::default(),
            TomlAssetPlugin::<Oscillators>::new(&["sh.toml"]),
            TomlAssetPlugin::<Settings>::new(&["settings.toml"]),
            ))
        .init_state::<AppState>()
        .add_systems(Startup, setup)
        .add_systems(Update, (pulse, animate_camera_and_thought, build.run_if(in_state(AppState::Building))))
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial,SphericalHarmonicsMaterial>>>,
) {
    let amplitudes_data = vec![0.0; NUM_COEFFICIENTS];

    let amplitudes = buffers.add(ShaderStorageBuffer::from(amplitudes_data));

    let material = SphericalHarmonicsMaterial { amplitudes };
    let material_handle = materials.add(ExtendedMaterial {
            base: StandardMaterial { ..Default::default() },
            extension: material,
        });
    commands.insert_resource(SphericalHarmonicsMaterialHandle(material_handle.clone()));

    let oscillator_handle = OscillatorsHandle(asset_server.load("thought.sh.toml"));
    commands.insert_resource(oscillator_handle);

    let settings_handle = SettingsHandle(asset_server.load("settings.toml"));
    commands.insert_resource(settings_handle);
}

fn build(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    settings: Res<Assets<Settings>>,
    settings_handle: Res<SettingsHandle>,
    material_handle: Res<SphericalHarmonicsMaterialHandle>,
    mut state: ResMut<NextState<AppState>>,
    asset_server: Res<AssetServer>,
    audio: Res<Audio>,
) {
    if let Some(settings) = settings.get(&settings_handle.0)
    {
        let material_handle = &material_handle.0;

        let mut sphere_mesh : Mesh = Mesh::from(Sphere::new(1.0).mesh().kind(SphereKind::Ico { subdivisions : settings.subdivisions }))
            .with_generated_tangents().expect("Could not generate tangents for sphere mesh! For some reason!");

        let sphere_mesh_vertices = sphere_mesh.count_vertices();

        sphere_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![Vec4::ZERO; sphere_mesh_vertices]);

        commands.spawn((
            Mesh3d(meshes.add(sphere_mesh)),
            MeshMaterial3d(material_handle.clone()),
            Transform::from_xyz(0.0,2.0,0.0),
        ));

        commands.spawn((
            DirectionalLight {
                shadows_enabled: true,
                shadow_depth_bias: settings.shadow_bias,
                ..default()
            },
            Transform::from_xyz(1.0, 1.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
        ));



        commands.spawn((SceneRoot(asset_server.load(
            GltfAssetLabel::Scene(0).from_asset("models/brain_200k.glb"))),
            Transform::from_translation(settings.brain_position.into()).with_scale(Vec3::splat(settings.brain_scale)),
        ));

        audio.play(
            asset_server.load("music/ALPA - 1 - Cute.mp3")
        );

        // camera
        commands.spawn((
            Camera3d::default(),
            Camera {
                hdr: true,
                ..default()
            },
            Transform::from_xyz(-4.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
            Tonemapping::TonyMcMapface,
            Bloom::NATURAL,
            DepthOfField {
                mode: DepthOfFieldMode::Bokeh,
                focal_distance: settings.initial_camera_distance,
                aperture_f_stops: settings.f_stops,
                ..default()
            }
        ));

        let ambient = settings.ambient_light;
        let ambient = Color::srgba(ambient[0], ambient[1], ambient[2], ambient[3]);

        commands.insert_resource(AmbientLight {
            color: ambient,
            brightness: settings.ambient_brightness,
        });

        commands.insert_resource(ClearColor ( ambient ));

        state.set(AppState::Running)
    }
    
}

fn clamp(x : f32, lowerlimit : f32, upperlimit : f32) -> f32
{
    if x < lowerlimit
    {
        return lowerlimit;
    }
    if x > upperlimit
    {
        return upperlimit;
    }
    x
}

fn smoothstep (edge0 : f32, edge1 : f32, x : f32) -> f32
{
    let x = clamp((x - edge0)/(edge1-edge0), 0., 1.);

    x * x * (3.0 - 2.0 * x)
}

fn pulse(
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
    material_handle: Res<SphericalHarmonicsMaterialHandle>,

    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial,SphericalHarmonicsMaterial>>>,
    time: Res<Time>,
    oscillators: Res<Assets<Oscillators>>,
    oscillator_handle: Res<OscillatorsHandle>,
    settings: Res<Assets<Settings>>,
    settings_handle: Res<SettingsHandle>,
){
    let material = materials.get_mut(&material_handle.0).unwrap();
    let oscillators = oscillators.get(&oscillator_handle.0).unwrap();
    let settings = settings.get(&settings_handle.0).unwrap();
    let buffer = buffers.get_mut(&material.extension.amplitudes).unwrap();
    buffer.set_data(
        (0..NUM_COEFFICIENTS)
            .map(|i| {
                let t = time.elapsed_secs();
                smoothstep(oscillators.start[i], oscillators.start[i] + settings.harmonic_spinup_time, t) * (oscillators.amplitudes[i] * ops::sin(settings.speed * oscillators.frequencies[i] * t + oscillators.phases[i])+oscillators.biases[i])
            })
            .collect::<Vec<f32>>()
            .as_slice(),
    );
}

fn animate_camera_and_thought(
    mut camera_query: Query<(&mut Transform, &mut DepthOfField), (With<Camera3d>, Without<Mesh3d>)>,
    mut thought_query: Query<&mut Transform, With<Mesh3d>>,
    settings: Res<Assets<Settings>>,
    settings_handle: Res<SettingsHandle>,
    time: Res<Time>
) {
    let settings = settings.get(&settings_handle.0).unwrap();

    let thought_height = (time.elapsed_secs() - settings.thought_appear_time) * settings.thought_speed + settings.thought_initial_height;

    let thought_height = max(FloatOrd(thought_height), FloatOrd(0.)).0;

    let thought_height = settings.slowdown_rate * thought_height/(settings.slowdown_rate+thought_height);

    let t = time.elapsed_secs();

    let camera_distance = settings.initial_camera_distance + (1. + thought_height);

    for mut transform in &mut thought_query {
        transform.translation = Vec3 {x : 0.0, y: thought_height, z: 0.0 };
    }

    let camera_rotation_time = max(FloatOrd(t - settings.camera_spin_time), FloatOrd(0.)).0;

    let camera_rotation_time = ((camera_rotation_time*settings.camera_spin_falloff + 0.25).sqrt() - 0.5)/settings.camera_spin_falloff;

    for (mut transform, mut dof) in &mut camera_query {
        let camera_rotation = settings.camera_rotation_speed * camera_rotation_time;
        transform.translation = Vec3 { x: camera_distance * sin(camera_rotation), y: (settings.slowdown_rate*t/(settings.slowdown_rate+t)) * settings.camera_speed - settings.initial_camera_distance, z: camera_distance * cos(camera_rotation)};
        transform.look_at(Vec3 {x : 0.0, y: 0.6 * thought_height, z: 0.0}, Vec3::Y);
        dof.focal_distance = camera_distance;
    }
}

#[derive(Resource)]
struct SphericalHarmonicsMaterialHandle(Handle<ExtendedMaterial<StandardMaterial, SphericalHarmonicsMaterial>>);

#[derive(Resource)]
struct OscillatorsHandle(Handle<Oscillators>);

#[derive(Resource)]
struct SettingsHandle(Handle<Settings>);

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct SphericalHarmonicsMaterial {
    #[storage(100, read_only)]
    amplitudes: Handle<ShaderStorageBuffer>,
}

#[derive(Deserialize, Asset, TypePath)]
struct Oscillators {
    amplitudes: Vec<f32>,
    frequencies: Vec<f32>,
    phases: Vec<f32>,
    start: Vec<f32>,
    biases: Vec<f32>,
}

#[derive(Deserialize, Asset, TypePath)]
struct Settings {
    subdivisions: u32,
    speed: f32,
    brain_position: [f32; 3],
    brain_scale: f32,
    shadow_bias: f32,
    ambient_light: [f32; 4],
    ambient_brightness: f32,
    camera_speed: f32,
    camera_rotation_speed: f32,
    initial_camera_distance: f32,
    camera_spin_time: f32,
    camera_spin_falloff: f32,
    harmonic_spinup_time: f32,
    thought_speed: f32,
    thought_appear_time : f32,
    thought_initial_height : f32,
    f_stops: f32,
    slowdown_rate: f32
}

impl MaterialExtension for SphericalHarmonicsMaterial {
    fn vertex_shader() -> ShaderRef {
        HARMONICS_SHADER_PATH.into()
    }

    fn fragment_shader() -> ShaderRef {
        HARMONICS_SHADER_PATH.into()
    }
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum AppState {
    #[default]
    Building,
    Running
}