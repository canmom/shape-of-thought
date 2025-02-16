use bevy::{
    prelude::*,
    reflect::TypePath,
    render::{
        render_resource::{AsBindGroup, ShaderRef},
        storage::ShaderStorageBuffer,
        mesh::SphereKind
    },
    pbr::{ExtendedMaterial,MaterialExtension},
};
use bevy_common_assets::toml::TomlAssetPlugin;
use serde::{Deserialize};

const HARMONICS_SHADER_PATH: &str = "shaders/sphericalharmonics.wgsl";
const NUM_COEFFICIENTS: usize = 9;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            MaterialPlugin::<ExtendedMaterial<StandardMaterial, SphericalHarmonicsMaterial>>::default(),
            TomlAssetPlugin::<Oscillators>::new(&["sh.toml"]),
            TomlAssetPlugin::<Settings>::new(&["settings.toml"]),
            ))
        .init_state::<AppState>()
        .add_systems(Startup, setup)
        .add_systems(Update, (pulse, build.run_if(in_state(AppState::Building))))
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

        // camera
        commands.spawn((
            Camera3d::default(),
            Transform::from_xyz(-4.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        ));

        state.set(AppState::Running)
    }
    
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
                oscillators.amplitudes[i] * ops::sin(settings.speed * oscillators.frequencies[i] * t + oscillators.phases[i])
            })
            .collect::<Vec<f32>>()
            .as_slice(),
    );
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
}

#[derive(Deserialize, Asset, TypePath)]
struct Settings {
    subdivisions: u32,
    speed: f32,
    brain_position: [f32; 3],
    brain_scale: f32,
    shadow_bias: f32
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