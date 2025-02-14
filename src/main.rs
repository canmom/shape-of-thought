use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderRef},
};

const HARMONICS_SHADER_PATH: &str = "shaders/sphericalharmonics.wgsl";

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, MaterialPlugin::<SphericalHarmonicsMaterial>::default()))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<SphericalHarmonicsMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Sphere::default())),
        MeshMaterial3d(materials.add(SphericalHarmonicsMaterial {})),
        Transform::from_xyz(0.0,0.0,0.0)
    ));

    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct SphericalHarmonicsMaterial {
}

impl Material for SphericalHarmonicsMaterial {
    fn vertex_shader() -> ShaderRef {
        HARMONICS_SHADER_PATH.into()
    }

    fn fragment_shader() -> ShaderRef {
        HARMONICS_SHADER_PATH.into()
    }
}