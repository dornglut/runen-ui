use glam::Vec3;

pub fn central_difference<E, F>(
    sample_signed_value: F,
    point: Vec3,
    epsilon: f32,
) -> Result<Vec3, E>
where
    F: Fn(Vec3) -> Result<f32, E>,
{
    let x = Vec3::new(epsilon, 0.0, 0.0);
    let y = Vec3::new(0.0, epsilon, 0.0);
    let z = Vec3::new(0.0, 0.0, epsilon);

    let dx = sample_signed_value(point + x)? - sample_signed_value(point - x)?;
    let dy = sample_signed_value(point + y)? - sample_signed_value(point - y)?;
    let dz = sample_signed_value(point + z)? - sample_signed_value(point - z)?;
    Ok(Vec3::new(dx, dy, dz) / (2.0 * epsilon))
}
