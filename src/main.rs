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
    window::WindowMode,
};
//use rand::prelude::*;
use bevy_common_assets::toml::TomlAssetPlugin;
use serde::{Deserialize};
use std::cmp::max;
use std::time::Duration;
use kira::tween::Easing;

const HARMONICS_SHADER_PATH: &str = "shaders/sphericalharmonics.wgsl";
const NUM_COEFFICIENTS: usize = 9;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    mode: if cfg!(target_os = "windows") {
                        WindowMode::BorderlessFullscreen(MonitorSelection::Primary)
                    } else {
                        WindowMode::Fullscreen(MonitorSelection::Primary)
                    },
                    ..default()
                }),
                ..default()
            }),
            AudioPlugin,
            MaterialPlugin::<ExtendedMaterial<StandardMaterial, SphericalHarmonicsMaterial>>::default(),
            TomlAssetPlugin::<Oscillators>::new(&["sh.toml"]),
            TomlAssetPlugin::<Settings>::new(&["settings.toml"]),
            ))
        .init_state::<AppState>()
        .add_systems(Startup, setup)
        .add_systems(Update, (pulse, animate_camera_and_thought, quit_after_time, build.run_if(in_state(AppState::Building))))
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
                let t = if cfg!(feature = "screenshot") {
                    settings.screenshot_time
                } else {
                    time.elapsed_secs()
                };
                smoothstep(oscillators.start[i], oscillators.start[i] + settings.harmonic_spinup_time, t) * (oscillators.amplitudes[i] * ops::sin(settings.speed * oscillators.frequencies[i] * t + oscillators.phases[i])+oscillators.biases[i])
            })
            .collect::<Vec<f32>>()
            .as_slice(),
    );
}

fn animate_camera_and_thought(
    mut camera_query: Query<(&mut Transform, &mut DepthOfField), (With<Camera3d>, Without<Mesh3d>)>,
    mut thought_query: Query<&mut Transform, (With<Mesh3d>, Without<Velocity>)>,
    settings: Res<Assets<Settings>>,
    settings_handle: Res<SettingsHandle>,
    time: Res<Time>
) {
    let settings = settings.get(&settings_handle.0).unwrap();

    let t = if cfg!(feature = "screenshot") {
        settings.screenshot_time
    } else {
        time.elapsed_secs()
    };

    let thought_height = (t - settings.thought_appear_time) * settings.thought_speed + settings.thought_initial_height;

    let thought_height = max(FloatOrd(thought_height), FloatOrd(0.)).0;

    let thought_height = settings.slowdown_rate * thought_height/(settings.slowdown_rate+thought_height);

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

// fn spawn_additional_thoughts(
//     commands: Commands,
//     mut meshes: ResMut<Assets<Mesh>>,
//     settings: Res<Assets<Settings>>,
//     settings_handle: Res<SettingsHandle>,
//     material_handle: Res<SphericalHarmonicsMaterialHandle>,
// ) {
//     let settings = settings.get(&settings_handle.0).unwrap();

//     let mut rng = rand::rng();

//     for i in 0..settings.num_additional_thoughts {

//     }
// }

fn quit_after_time(
    mut app_exit_events: ResMut<Events<bevy::app::AppExit>>,
    time : Res<Time>,
    settings: Res<Assets<Settings>>,
    settings_handle: Res<SettingsHandle>,
    audio: Res<Audio>,
    current_state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let settings = settings.get(&settings_handle.0).unwrap();

    if time.elapsed_secs() > settings.end_time - 5.0 && *current_state.get() != AppState::Quitting {
        audio.pause().fade_out(AudioTween::new(Duration::from_secs(5), Easing::Linear));
        next_state.set(AppState::Quitting);
    }

    if time.elapsed_secs() > settings.end_time {
         app_exit_events.send(AppExit::Success);
    }
}

#[derive(Debug, Component, Clone, Copy, PartialEq, Default, Deref, DerefMut)]
struct Velocity(Vec3);

#[derive(Debug, Component, Clone, Copy, PartialEq, Default, Deref, DerefMut)]
struct AngularVelocity(Vec3);

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
    slowdown_rate: f32,
    additional_thoughts_time: f32,
    num_additional_thoughts: usize,
    end_time: f32,
    screenshot_time: f32,
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
    Running,
    Quitting,
}