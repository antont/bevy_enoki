use super::{prelude::EmissionShape, Particle2dEffect, ParticleEffectHandle};
use crate::values::Random;
use bevy_asset::Assets;
use bevy_camera::primitives::Aabb;
use bevy_color::LinearRgba;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{Added, Without},
    reflect::ReflectComponent,
    system::{Commands, Query, Res},
};
use bevy_math::{EulerRot, Vec2, Vec3};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_time::{Time, Timer, TimerMode, Virtual};
use bevy_transform::components::{GlobalTransform, Transform};
use std::time::Duration;
use wide::f32x8;

/// Tag Component, deactivates spawner after the first
/// spawning of particles
#[derive(Component, Default, Debug, Reflect, Clone)]
#[reflect(Component, Debug, Default)]
pub enum OneShot {
    #[default]
    Deactivate,
    Despawn,
}

/// Spawner states controls the spawner
#[derive(Component, Clone, Debug, Reflect)]
pub struct ParticleSpawnerState {
    pub max_particles: u32,
    pub active: bool,
    pub timer: Timer,
    pub previous_position: Option<Vec3>,
}

/// A clone of the asset, unique to each spawner
/// that can be mutated by any system.
/// Any change to `Particle2dEffect` Asset will
/// re-copy the effect.
#[derive(Component, Deref, DerefMut, Default)]
pub struct ParticleEffectInstance(pub Option<Particle2dEffect>);

impl Default for ParticleSpawnerState {
    fn default() -> Self {
        Self {
            active: true,
            max_particles: u32::MAX,
            timer: Timer::new(Duration::ZERO, TimerMode::Repeating),
            previous_position: None,
        }
    }
}

/// Per-particle values drawn at spawn from the effect's `scale_spread`, `color_spread` and `orbit`.
#[derive(Clone, Copy, Default, Debug)]
pub(crate) struct SpawnExtras {
    scale: (f32, f32),
    color: ([f32; 4], [f32; 4]),
    /// Angle, radius, radius change per second, turn rate (radians per second) of an orbiting particle.
    orbit: [f32; 4],
    /// Rotation at the start and at the end of life.
    rotation: (f32, f32),
}

/// Component for storing particle data
#[derive(Component, Default, Clone, Reflect)]
pub struct ParticleStore {
    pub(crate) position_x: Vec<f32>,
    pub(crate) position_y: Vec<f32>,
    pub(crate) position_z: Vec<f32>,
    pub(crate) rotation: Vec<f32>,
    pub(crate) scale_x: Vec<f32>,
    pub(crate) scale_y: Vec<f32>,
    pub(crate) scale_z: Vec<f32>,
    pub(crate) duration: Vec<f32>,
    pub(crate) duration_fraction: Vec<f32>,
    pub(crate) velocity_x: Vec<f32>,
    pub(crate) velocity_y: Vec<f32>,
    pub(crate) velocity_z: Vec<f32>,
    pub(crate) angular_velocity: Vec<f32>,
    pub(crate) color_r: Vec<f32>,
    pub(crate) color_g: Vec<f32>,
    pub(crate) color_b: Vec<f32>,
    pub(crate) color_a: Vec<f32>,
    pub(crate) frame: Vec<u32>,
    pub(crate) linear_acceleration: Vec<f32>,
    pub(crate) linear_damp: Vec<f32>,
    pub(crate) angular_acceleration: Vec<f32>,
    pub(crate) angular_damp: Vec<f32>,
    pub(crate) gravity_speed: Vec<f32>,
    pub(crate) gravity_x: Vec<f32>,
    pub(crate) gravity_y: Vec<f32>,
    pub(crate) gravity_z: Vec<f32>,
    pub(crate) origin_x: Vec<f32>,
    pub(crate) origin_y: Vec<f32>,
    pub(crate) radial_acceleration: Vec<f32>,
    pub(crate) tangential_acceleration: Vec<f32>,
    pub(crate) scale_offset_start: Vec<f32>,
    pub(crate) scale_offset_end: Vec<f32>,
    pub(crate) color_offset_start: Vec<[f32; 4]>,
    pub(crate) color_offset_end: Vec<[f32; 4]>,
    pub(crate) orbit_angle: Vec<f32>,
    pub(crate) orbit_radius: Vec<f32>,
    pub(crate) orbit_delta_radius: Vec<f32>,
    pub(crate) orbit_turn_rate: Vec<f32>,
    pub(crate) rotation_start: Vec<f32>,
    pub(crate) rotation_end: Vec<f32>,
}

impl ParticleStore {
    pub fn len(&self) -> usize {
        self.duration.len()
    }

    pub fn is_empty(&self) -> bool {
        self.duration.is_empty()
    }

    pub fn clear(&mut self) {
        macro_rules! clear {
            ($($field:ident),+ $(,)?) => {
                $(self.$field.clear();)+
            };
        }
        clear!(
            position_x,
            position_y,
            position_z,
            rotation,
            scale_x,
            scale_y,
            scale_z,
            duration,
            duration_fraction,
            velocity_x,
            velocity_y,
            velocity_z,
            angular_velocity,
            color_r,
            color_g,
            color_b,
            color_a,
            frame,
            linear_acceleration,
            linear_damp,
            angular_acceleration,
            angular_damp,
            gravity_speed,
            gravity_x,
            gravity_y,
            gravity_z,
            origin_x,
            origin_y,
            radial_acceleration,
            tangential_acceleration,
            scale_offset_start,
            scale_offset_end,
            color_offset_start,
            color_offset_end,
            orbit_angle,
            orbit_radius,
            orbit_delta_radius,
            orbit_turn_rate,
            rotation_start,
            rotation_end,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn push(
        &mut self,
        transform: Transform,
        duration: f32,
        velocity: Vec3,
        angular_velocity: f32,
        color: LinearRgba,
        linear_acceleration: f32,
        linear_damp: f32,
        angular_acceleration: f32,
        angular_damp: f32,
        gravity_speed: f32,
        gravity_direction: Vec3,
        origin: Vec2,
        radial_acceleration: f32,
        tangential_acceleration: f32,
        offsets: SpawnExtras,
    ) {
        self.position_x.push(transform.translation.x);
        self.position_y.push(transform.translation.y);
        self.position_z.push(transform.translation.z);
        self.rotation
            .push(transform.rotation.to_euler(EulerRot::XYZ).2);
        self.scale_x.push(transform.scale.x);
        self.scale_y.push(transform.scale.y);
        self.scale_z.push(transform.scale.z);
        self.duration.push(duration);
        self.duration_fraction.push(0.0);
        self.velocity_x.push(velocity.x);
        self.velocity_y.push(velocity.y);
        self.velocity_z.push(velocity.z);
        self.angular_velocity.push(angular_velocity);
        self.color_r.push(color.red);
        self.color_g.push(color.green);
        self.color_b.push(color.blue);
        self.color_a.push(color.alpha);
        self.frame.push(0);
        self.linear_acceleration.push(linear_acceleration);
        self.linear_damp.push(linear_damp);
        self.angular_acceleration.push(angular_acceleration);
        self.angular_damp.push(angular_damp);
        self.gravity_speed.push(gravity_speed);
        self.gravity_x.push(gravity_direction.x);
        self.gravity_y.push(gravity_direction.y);
        self.gravity_z.push(gravity_direction.z);
        self.origin_x.push(origin.x);
        self.origin_y.push(origin.y);
        self.radial_acceleration.push(radial_acceleration);
        self.tangential_acceleration.push(tangential_acceleration);
        self.scale_offset_start.push(offsets.scale.0);
        self.scale_offset_end.push(offsets.scale.1);
        self.color_offset_start.push(offsets.color.0);
        self.color_offset_end.push(offsets.color.1);
        self.orbit_angle.push(offsets.orbit[0]);
        self.orbit_radius.push(offsets.orbit[1]);
        self.orbit_delta_radius.push(offsets.orbit[2]);
        self.orbit_turn_rate.push(offsets.orbit[3]);
        self.rotation_start.push(offsets.rotation.0);
        self.rotation_end.push(offsets.rotation.1);
    }

    fn swap_remove(&mut self, index: usize) {
        macro_rules! remove {
            ($($field:ident),+ $(,)?) => {
                $(self.$field.swap_remove(index);)+
            };
        }
        remove!(
            position_x,
            position_y,
            position_z,
            rotation,
            scale_x,
            scale_y,
            scale_z,
            duration,
            duration_fraction,
            velocity_x,
            velocity_y,
            velocity_z,
            angular_velocity,
            color_r,
            color_g,
            color_b,
            color_a,
            frame,
            linear_acceleration,
            linear_damp,
            angular_acceleration,
            angular_damp,
            gravity_speed,
            gravity_x,
            gravity_y,
            gravity_z,
            origin_x,
            origin_y,
            radial_acceleration,
            tangential_acceleration,
            scale_offset_start,
            scale_offset_end,
            color_offset_start,
            color_offset_end,
            orbit_angle,
            orbit_radius,
            orbit_delta_radius,
            orbit_turn_rate,
            rotation_start,
            rotation_end,
        );
    }

    fn remove_expired(&mut self) {
        for index in (0..self.len()).rev() {
            if self.duration_fraction[index] >= 1.0 {
                self.swap_remove(index);
            }
        }
    }
}

pub(crate) fn clone_effect(
    mut particle_spawners: Query<
        (&mut ParticleEffectInstance, &ParticleEffectHandle),
        Added<ParticleSpawnerState>,
    >,
    effects: Res<Assets<Particle2dEffect>>,
) {
    particle_spawners
        .iter_mut()
        .for_each(|(mut effect_overwrites, handle)| {
            let Some(effect) = effects.get(&handle.0) else {
                return;
            };

            effect_overwrites.0 = Some(effect.clone());
        });
}

pub(crate) fn remove_finished_spawner(
    mut cmd: Commands,
    spawner: Query<(Entity, &ParticleStore, &ParticleSpawnerState, &OneShot)>,
) {
    spawner
        .iter()
        .for_each(|(entity, store, controller, one_shot)| {
            if matches!(one_shot, OneShot::Despawn) && !controller.active && store.is_empty() {
                cmd.entity(entity).try_despawn();
            }
        })
}

pub(crate) fn update_spawner(
    mut particles: Query<(
        Entity,
        &mut ParticleStore,
        &mut ParticleSpawnerState,
        &ParticleEffectInstance,
        &GlobalTransform,
    )>,
    one_shots: Query<&OneShot>,
    time: Res<Time<Virtual>>,
) {
    particles.par_iter_mut().for_each(
        |(entity, mut store, mut state, effect_instance, transform)| {
            if state.max_particles <= store.len() as u32 {
                return;
            }

            let Some(effect) = &effect_instance.0 else {
                return;
            };

            let transform = transform.compute_transform();

            state
                .timer
                .set_duration(Duration::from_secs_f32(effect.spawn_rate));
            state.timer.tick(time.delta());

            if state.timer.is_finished() && state.active {
                for _ in 0..effect.spawn_amount {
                    create_particle(&mut store, effect, &transform);
                }

                if one_shots.get(entity).is_ok() {
                    state.active = false;
                }
            }
            let delta = time.delta_secs();
            let spawner_world_pos = transform.translation;

            // Handle relative positioning
            let position_delta = if effect.relative_positioning.unwrap_or(false) {
                let current_pos = spawner_world_pos;
                let delta = if let Some(prev_pos) = state.previous_position {
                    current_pos - prev_pos
                } else {
                    Vec3::ZERO
                };
                state.previous_position = Some(current_pos);
                delta
            } else {
                Vec3::ZERO
            };

            update_particles(&mut store, effect, delta, spawner_world_pos, position_delta);
            store.remove_expired();
        },
    );
}

fn update_particles(
    particles: &mut ParticleStore,
    effect: &Particle2dEffect,
    delta: f32,
    spawner_world_pos: Vec3,
    position_delta: Vec3,
) {
    update_particles_simd(particles, effect, delta, spawner_world_pos, position_delta);
}

fn create_particle(store: &mut ParticleStore, effect: &Particle2dEffect, transform: &Transform) {
    // direction
    let direction = effect
        .direction
        .as_ref()
        .map(|m| m.rand())
        .unwrap_or_default();

    // apply local rotation
    let direction = direction.rotate(transform.right().truncate());

    // speed
    let speed = effect
        .linear_speed
        .as_ref()
        .map(|s| s.rand())
        .unwrap_or_default();
    // angular
    let angular = effect
        .angular_speed
        .as_ref()
        .map(|s| s.rand())
        .unwrap_or_default();
    // angular
    let scale = effect.scale.as_ref().map(|s| s.rand()).unwrap_or_default();

    let gravity_direction = effect
        .gravity_direction
        .as_ref()
        .map(|g| g.rand())
        .unwrap_or_default()
        .extend(0.);

    let gravity_speed = effect
        .gravity_speed
        .as_ref()
        .map(|g| g.rand())
        .unwrap_or_default();

    let linear_damp = effect
        .linear_damp
        .as_ref()
        .map(|d| d.rand())
        .unwrap_or_default();

    let angular_damp = effect
        .angular_damp
        .as_ref()
        .map(|a| a.rand())
        .unwrap_or_default();

    let angular_acceleration = effect
        .angular_acceleration
        .as_ref()
        .map(|a| a.rand())
        .unwrap_or_default();

    let linear_acceleration = effect
        .linear_acceleration
        .as_ref()
        .map(|a| a.rand())
        .unwrap_or_default();

    let radial_acceleration = effect
        .radial_acceleration
        .as_ref()
        .map(|a| a.rand())
        .unwrap_or_default();

    let tangential_acceleration = effect
        .tangential_acceleration
        .as_ref()
        .map(|a| a.rand())
        .unwrap_or_default();

    let lifetime = effect.lifetime.rand();
    let spread4 = |spread: &LinearRgba| {
        let r = || rand::random::<f32>() * 2. - 1.;
        [
            spread.red * r(),
            spread.green * r(),
            spread.blue * r(),
            spread.alpha * r(),
        ]
    };
    let offsets = SpawnExtras {
        scale: effect
            .scale_spread
            .map(|(start, end)| {
                (
                    start * (rand::random::<f32>() * 2. - 1.),
                    end * (rand::random::<f32>() * 2. - 1.),
                )
            })
            .unwrap_or_default(),
        color: effect
            .color_spread
            .map(|(start, end)| (spread4(&start), spread4(&end)))
            .unwrap_or_default(),
        rotation: effect
            .rotation_over_life
            .map(|(start, end)| (start.rand(), end.rand()))
            .unwrap_or_default(),
        orbit: effect.orbit.as_ref().map_or([0.0; 4], |orbit| {
            // cocos2d radius mode `initParticle`: the radius runs from start to end over the particle's life.
            let (start, end) = (orbit.start_radius.rand(), orbit.end_radius.rand());
            [
                direction.y.atan2(direction.x),
                start,
                (end - start) / lifetime.max(f32::EPSILON),
                orbit.turn_rate.rand(),
            ]
        }),
    };

    let origin = transform.translation.truncate();
    let mut transform = *transform;
    transform.scale = Vec3::splat(scale);

    transform.translation += match effect.emission_shape {
        EmissionShape::Point => Vec3::ZERO,
        EmissionShape::Circle(radius) => {
            Vec3::new(rand::random::<f32>() - 0.5, rand::random::<f32>() - 0.5, 0.)
                .normalize_or_zero()
                * radius
                * rand::random::<f32>()
        }
        EmissionShape::Rect(half) => EmissionShape::rect_offset(
            half,
            rand::random::<f32>(),
            rand::random::<f32>(),
        )
        .extend(0.),
    };

    store.push(
        transform,
        lifetime,
        (direction * speed).extend(0.),
        angular,
        effect.color.unwrap_or(LinearRgba::WHITE),
        linear_acceleration,
        linear_damp,
        angular_acceleration,
        angular_damp,
        gravity_speed,
        gravity_direction,
        origin,
        radial_acceleration,
        tangential_acceleration,
        offsets,
    );
}

fn load8(values: &[f32], index: usize) -> f32x8 {
    f32x8::new(values[index..index + 8].try_into().unwrap())
}

fn store8(values: &mut [f32], index: usize, value: f32x8) {
    values[index..index + 8].copy_from_slice(&value.to_array());
}

fn update_particles_simd(
    particles: &mut ParticleStore,
    effect: &Particle2dEffect,
    delta: f32,
    spawner_world_pos: Vec3,
    position_delta: Vec3,
) {
    let delta8 = f32x8::splat(delta);
    let zero = f32x8::ZERO;
    let one = f32x8::ONE;
    let simd_len = particles.len() / 8 * 8;

    for index in (0..simd_len).step_by(8) {
        let mut px = load8(&particles.position_x, index) + f32x8::splat(position_delta.x);
        let mut py = load8(&particles.position_y, index) + f32x8::splat(position_delta.y);
        let mut pz = load8(&particles.position_z, index) + f32x8::splat(position_delta.z);
        let progress =
            load8(&particles.duration_fraction, index) + delta8 / load8(&particles.duration, index);

        let linear_factor = one
            + progress
                * (load8(&particles.linear_acceleration, index)
                    - load8(&particles.linear_damp, index))
                * delta8;
        let mut vx = load8(&particles.velocity_x, index) * linear_factor;
        let mut vy = load8(&particles.velocity_y, index) * linear_factor;
        let mut vz = load8(&particles.velocity_z, index) * linear_factor;

        let angular_factor = one
            + progress
                * (load8(&particles.angular_acceleration, index)
                    - load8(&particles.angular_damp, index))
                * delta8;
        let angular_velocity = load8(&particles.angular_velocity, index) * angular_factor;

        if let Some(attractors) = &effect.attractors {
            for attractor in attractors {
                let attractor_position = spawner_world_pos + attractor.position.extend(0.0);
                let dx = f32x8::splat(attractor_position.x) - px;
                let dy = f32x8::splat(attractor_position.y) - py;
                let dz = f32x8::splat(attractor_position.z) - pz;
                let distance_squared = dx * dx + dy * dy + dz * dz;
                let non_zero = distance_squared.simd_gt(zero);
                let safe_distance_squared = non_zero.select(distance_squared, one);
                let force = f32x8::splat(attractor.strength)
                    / safe_distance_squared.max(f32x8::splat(
                        attractor.min_distance * attractor.min_distance,
                    ))
                    * delta8
                    / safe_distance_squared.sqrt();
                let force = non_zero.select(force, zero);
                vx += dx * force;
                vy += dy * force;
                vz += dz * force;
            }
        }

        // Radial and tangential acceleration, relative to the spawn origin (cocos2d: `pos` is relative to it).
        let ox = load8(&particles.origin_x, index) + f32x8::splat(position_delta.x);
        let oy = load8(&particles.origin_y, index) + f32x8::splat(position_delta.y);
        let rx = px - ox;
        let ry = py - oy;
        let radial_len_squared = rx * rx + ry * ry;
        let has_radial = radial_len_squared.simd_gt(zero);
        let inverse_len = has_radial.select(one / radial_len_squared.sqrt(), zero);
        let (radial_x, radial_y) = (rx * inverse_len, ry * inverse_len);
        let radial_acceleration = load8(&particles.radial_acceleration, index);
        let tangential_acceleration = load8(&particles.tangential_acceleration, index);
        vx += (radial_x * radial_acceleration - radial_y * tangential_acceleration) * delta8;
        vy += (radial_y * radial_acceleration + radial_x * tangential_acceleration) * delta8;

        // Gravity accelerates the velocity (cocos2d: `dir += gravity * dt`).
        let gravity = load8(&particles.gravity_speed, index) * delta8;
        vx += load8(&particles.gravity_x, index) * gravity;
        vy += load8(&particles.gravity_y, index) * gravity;
        vz += load8(&particles.gravity_z, index) * gravity;
        px += vx * delta8;
        py += vy * delta8;
        pz += vz * delta8;
        let rotation = load8(&particles.rotation, index) + angular_velocity * delta8;

        store8(&mut particles.origin_x, index, ox);
        store8(&mut particles.origin_y, index, oy);
        store8(&mut particles.position_x, index, px);
        store8(&mut particles.position_y, index, py);
        store8(&mut particles.position_z, index, pz);
        store8(&mut particles.duration_fraction, index, progress);
        store8(&mut particles.velocity_x, index, vx);
        store8(&mut particles.velocity_y, index, vy);
        store8(&mut particles.velocity_z, index, vz);
        store8(&mut particles.angular_velocity, index, angular_velocity);
        store8(&mut particles.rotation, index, rotation);
    }

    for index in simd_len..particles.len() {
        update_particle_scalar(
            particles,
            effect,
            index,
            delta,
            spawner_world_pos,
            position_delta,
        );
    }

    if effect.orbit.is_some() {
        // cocos2d radius mode `update:`: the particle sits on a spiral around its spawn origin.
        for index in 0..particles.len() {
            particles.orbit_angle[index] += particles.orbit_turn_rate[index] * delta;
            particles.orbit_radius[index] += particles.orbit_delta_radius[index] * delta;
            let (sin, cos) = particles.orbit_angle[index].sin_cos();
            particles.position_x[index] = particles.origin_x[index] - cos * particles.orbit_radius[index];
            particles.position_y[index] = particles.origin_y[index] - sin * particles.orbit_radius[index];
        }
    }

    if effect.rotation_over_life.is_some() {
        // cocos2d: the rotation runs linearly from its start to its end value over the particle's life.
        for index in 0..particles.len() {
            let t = particles.duration_fraction[index].min(1.0);
            particles.rotation[index] = particles.rotation_start[index]
                + (particles.rotation_end[index] - particles.rotation_start[index]) * t;
        }
    }

    if let Some(scale_curve) = effect.scale_curve.as_ref() {
        for index in 0..particles.len() {
            let progress = particles.duration_fraction[index];
            let mut scale = scale_curve.lerp(progress);
            if effect.scale_spread.is_some() {
                // cocos2d: start and end size each get their own spread; the size runs linearly between them, never below 0.
                let t = progress.min(1.0);
                scale = (scale
                    + particles.scale_offset_start[index] * (1.0 - t)
                    + particles.scale_offset_end[index] * t)
                    .max(0.0);
            }
            particles.scale_x[index] = scale;
            particles.scale_y[index] = scale;
            particles.scale_z[index] = scale;
        }
    }

    if let Some(color_curve) = effect.color_curve.as_ref() {
        for index in 0..particles.len() {
            let progress = particles.duration_fraction[index];
            let mut color = color_curve.lerp(progress);
            if effect.color_spread.is_some() {
                // cocos2d: start and end colour each get their own spread; the colour runs linearly between them, within 0..1.
                let t = progress.min(1.0);
                let (start, end) = (
                    particles.color_offset_start[index],
                    particles.color_offset_end[index],
                );
                let mix = |channel: usize, base: f32| {
                    (base + start[channel] * (1.0 - t) + end[channel] * t).clamp(0.0, 1.0)
                };
                color = LinearRgba::new(
                    mix(0, color.red),
                    mix(1, color.green),
                    mix(2, color.blue),
                    mix(3, color.alpha),
                );
            }
            particles.color_r[index] = color.red;
            particles.color_g[index] = color.green;
            particles.color_b[index] = color.blue;
            particles.color_a[index] = color.alpha;
        }
    }
}

fn update_particle_scalar(
    particles: &mut ParticleStore,
    effect: &Particle2dEffect,
    index: usize,
    delta: f32,
    spawner_world_pos: Vec3,
    position_delta: Vec3,
) {
    particles.position_x[index] += position_delta.x;
    particles.position_y[index] += position_delta.y;
    particles.position_z[index] += position_delta.z;
    particles.duration_fraction[index] += delta / particles.duration[index];
    let progress = particles.duration_fraction[index];

    let linear_factor = 1.0
        + progress * (particles.linear_acceleration[index] - particles.linear_damp[index]) * delta;
    particles.velocity_x[index] *= linear_factor;
    particles.velocity_y[index] *= linear_factor;
    particles.velocity_z[index] *= linear_factor;

    let angular_factor = 1.0
        + progress
            * (particles.angular_acceleration[index] - particles.angular_damp[index])
            * delta;
    particles.angular_velocity[index] *= angular_factor;

    if let Some(attractors) = &effect.attractors {
        for attractor in attractors {
            let attractor_position = spawner_world_pos + attractor.position.extend(0.0);
            let dx = attractor_position.x - particles.position_x[index];
            let dy = attractor_position.y - particles.position_y[index];
            let dz = attractor_position.z - particles.position_z[index];
            let distance_squared = dx * dx + dy * dy + dz * dz;

            if distance_squared > 0.0 {
                let force = attractor.strength
                    / distance_squared.max(attractor.min_distance * attractor.min_distance)
                    * delta
                    / distance_squared.sqrt();
                particles.velocity_x[index] += dx * force;
                particles.velocity_y[index] += dy * force;
                particles.velocity_z[index] += dz * force;
            }
        }
    }

    // Radial and tangential acceleration, relative to the spawn origin (cocos2d: `pos` is relative to it).
    particles.origin_x[index] += position_delta.x;
    particles.origin_y[index] += position_delta.y;
    let radial = Vec2::new(
        particles.position_x[index] - particles.origin_x[index],
        particles.position_y[index] - particles.origin_y[index],
    )
    .normalize_or_zero();
    let tangential = Vec2::new(-radial.y, radial.x);
    let acceleration = radial * particles.radial_acceleration[index]
        + tangential * particles.tangential_acceleration[index];
    particles.velocity_x[index] += acceleration.x * delta;
    particles.velocity_y[index] += acceleration.y * delta;

    // Gravity accelerates the velocity (cocos2d: `dir += gravity * dt`).
    let gravity = particles.gravity_speed[index] * delta;
    particles.velocity_x[index] += particles.gravity_x[index] * gravity;
    particles.velocity_y[index] += particles.gravity_y[index] * gravity;
    particles.velocity_z[index] += particles.gravity_z[index] * gravity;
    particles.position_x[index] += particles.velocity_x[index] * delta;
    particles.position_y[index] += particles.velocity_y[index] * delta;
    particles.position_z[index] += particles.velocity_z[index] * delta;

    particles.rotation[index] += particles.angular_velocity[index] * delta;
}

pub(crate) fn calculate_particle_bounds(
    mut cmd: Commands,
    spawners: Query<(Entity, &ParticleStore, &GlobalTransform), Without<crate::NoAutoAabb>>,
) {
    spawners.iter().for_each(|(entity, store, transform)| {
        if store.is_empty() {
            return;
        }
        let accuracy = (store.len() / 1000).clamp(1, 10);

        let (min, max) =
            (0..store.len())
                .step_by(accuracy)
                .fold((Vec2::MAX, Vec2::MIN), |mut acc, index| {
                    acc.0.x = acc.0.x.min(store.position_x[index]);
                    acc.0.y = acc.0.y.min(store.position_y[index]);
                    acc.1.x = acc.1.x.max(store.position_x[index]);
                    acc.1.y = acc.1.y.max(store.position_y[index]);
                    acc
                });

        let mut aabb = Aabb::from_min_max(min.extend(0.), max.extend(0.));
        aabb.center -= transform.translation().to_vec3a();

        cmd.entity(entity).try_insert(aabb);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::MultiCurve;
    use crate::values::{Rval, Spread};
    use std::{
        hint::black_box,
        time::{Duration, Instant},
    };

    fn particle_store(count: usize) -> ParticleStore {
        let mut particles = ParticleStore::default();
        for _ in 0..count {
            particles.push(
                Transform::default(),
                1000.0,
                Vec3::new(10.0, 5.0, 0.0),
                0.5,
                LinearRgba::WHITE,
                0.2,
                0.1,
                0.2,
                0.1,
                9.81,
                Vec3::NEG_Y,
                Vec2::ZERO,
                0.0,
                0.0,
                SpawnExtras::default(),
            );
        }
        particles
    }

    #[test]
    fn simd_matches_scalar_update() {
        let effect = Particle2dEffect::default();
        let mut simd = particle_store(8);
        let mut scalar = simd.clone();
        let delta = 1.0 / 60.0;

        update_particles(&mut simd, &effect, delta, Vec3::ZERO, Vec3::ZERO);
        for index in 0..scalar.len() {
            update_particle_scalar(&mut scalar, &effect, index, delta, Vec3::ZERO, Vec3::ZERO);
        }

        for index in 0..simd.len() {
            for (actual, expected) in [
                (simd.position_x[index], scalar.position_x[index]),
                (simd.position_y[index], scalar.position_y[index]),
                (simd.velocity_x[index], scalar.velocity_x[index]),
                (simd.velocity_y[index], scalar.velocity_y[index]),
                (
                    simd.duration_fraction[index],
                    scalar.duration_fraction[index],
                ),
                (simd.rotation[index], scalar.rotation[index]),
            ] {
                assert!((actual - expected).abs() < 1e-5, "{actual} != {expected}");
            }
        }
    }

    /// cocos2d `CCParticleSystem.m [update:]`: `dir += gravity * dt; pos += dir * dt`.
    fn cocos_gravity_steps(gravity: f32, dt: f32, steps: usize) -> (f32, f32) {
        let (mut velocity, mut position) = (0.0_f32, 0.0_f32);
        for _ in 0..steps {
            velocity += gravity * dt;
            position += velocity * dt;
        }
        (velocity, position)
    }

    /// cocos2d `update:` gravity mode: `radial = normalize(pos); tangential = (-radial.y, radial.x);
    /// dir += (radial * radialAccel + tangential * tangentialAccel) * dt; pos += dir * dt`, `pos` relative to the emitter.
    fn cocos_radial_steps(start: Vec2, radial_accel: f32, tangential_accel: f32, dt: f32, steps: usize) -> (Vec2, Vec2) {
        let (mut pos, mut dir) = (start, Vec2::ZERO);
        for _ in 0..steps {
            let radial = pos.normalize_or_zero();
            let tangential = Vec2::new(-radial.y, radial.x);
            dir += (radial * radial_accel + tangential * tangential_accel) * dt;
            pos += dir * dt;
        }
        (pos, dir)
    }

    #[test]
    fn radial_and_tangential_acceleration_follow_cocos2d() {
        let origin = Vec2::new(100.0, -20.0);
        let start = Vec2::new(3.0, 4.0);
        let (dt, steps) = (1.0 / 30.0, 12);
        let (expected_pos, expected_dir) = cocos_radial_steps(start, -200.0, 90.0, dt, steps);
        let effect = Particle2dEffect::default();

        // 9 particles: 8 on the SIMD path, the last on the scalar path.
        let mut particles = ParticleStore::default();
        for _ in 0..9 {
            particles.push(
                Transform::from_translation((origin + start).extend(0.0)),
                1000.0,
                Vec3::ZERO,
                0.0,
                LinearRgba::WHITE,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                Vec3::ZERO,
                origin,
                -200.0,
                90.0,
                SpawnExtras::default(),
            );
        }
        for _ in 0..steps {
            update_particles(&mut particles, &effect, dt, Vec3::ZERO, Vec3::ZERO);
        }
        for index in [0, 8] {
            let pos = Vec2::new(particles.position_x[index], particles.position_y[index]) - origin;
            let dir = Vec2::new(particles.velocity_x[index], particles.velocity_y[index]);
            assert!(pos.distance(expected_pos) < 1e-2, "{index}: {pos} != {expected_pos}");
            assert!(dir.distance(expected_dir) < 1e-2, "{index}: {dir} != {expected_dir}");
        }
    }

    #[test]
    fn a_particle_at_its_origin_gets_no_radial_push() {
        let mut particles = ParticleStore::default();
        particles.push(
            Transform::default(),
            1000.0,
            Vec3::ZERO,
            0.0,
            LinearRgba::WHITE,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            Vec3::ZERO,
            Vec2::ZERO,
            500.0,
            500.0,
            SpawnExtras::default(),
        );
        update_particles(&mut particles, &Particle2dEffect::default(), 0.1, Vec3::ZERO, Vec3::ZERO);
        assert!(particles.position_x[0].is_finite() && particles.position_x[0] == 0.0);
    }

    #[test]
    fn spawn_records_the_origin_and_draws_the_acceleration_spread() {
        let effect = Particle2dEffect {
            radial_acceleration: Some(Spread(0.0, 5.0)),
            tangential_acceleration: Some(Spread(10.0, 0.0)),
            emission_shape: EmissionShape::Rect(Vec2::new(8.0, 8.0)),
            ..Default::default()
        };
        let mut store = ParticleStore::default();
        let at = Transform::from_xyz(50.0, 60.0, 0.0);
        for _ in 0..100 {
            create_particle(&mut store, &effect, &at);
        }
        assert!(store.origin_x.iter().all(|x| *x == 50.0) && store.origin_y.iter().all(|y| *y == 60.0));
        assert!(store.radial_acceleration.iter().all(|a| a.abs() <= 5.0));
        assert!(store.radial_acceleration.iter().any(|a| *a > 1.0) && store.radial_acceleration.iter().any(|a| *a < -1.0));
        assert!(store.tangential_acceleration.iter().all(|a| *a == 10.0));
    }

    /// cocos2d `initParticle`: `size = start + startVar * r1`, `end = end + endVar * r2`, linear between, never below 0.
    #[test]
    fn scale_spread_gives_each_particle_its_own_start_and_end() {
        let effect = Particle2dEffect {
            scale_curve: Some(MultiCurve::new().with_point(40.0, 0.0, None).with_point(0.0, 1.0, None)),
            scale_spread: Some((30.0, 20.0)),
            ..Default::default()
        };
        let mut store = ParticleStore::default();
        for _ in 0..400 {
            create_particle(&mut store, &effect, &Transform::default());
        }
        let starts: Vec<f32> = store.scale_offset_start.iter().map(|o| 40.0 + o).collect();
        assert!(starts.iter().all(|s| (10.0..=70.0).contains(s)));
        assert!(starts.iter().any(|s| *s < 20.0) && starts.iter().any(|s| *s > 60.0));

        // At the start of life a particle is `40 + offset`, at the end `0 + end offset` floored at 0.
        update_particles(&mut store, &effect, 0.0, Vec3::ZERO, Vec3::ZERO);
        for i in 0..store.len() {
            assert!((store.scale_x[i] - (40.0 + store.scale_offset_start[i]).max(0.0)).abs() < 1e-3);
        }
        for fraction in store.duration_fraction.iter_mut() {
            *fraction = 1.0;
        }
        update_particles(&mut store, &effect, 0.0, Vec3::ZERO, Vec3::ZERO);
        for i in 0..store.len() {
            assert!((store.scale_x[i] - store.scale_offset_end[i].max(0.0)).abs() < 1e-3);
            assert!(store.scale_x[i] >= 0.0);
        }
        assert!(store.scale_x.iter().any(|s| *s == 0.0), "negative end sizes are floored at 0");
    }

    #[test]
    fn color_spread_varies_per_particle_and_stays_in_range() {
        let curve = MultiCurve::new()
            .with_point(LinearRgba::new(0.9, 0.0, 0.0, 1.0), 0.0, None)
            .with_point(LinearRgba::new(0.0, 0.0, 0.0, 1.0), 1.0, None);
        let spread = LinearRgba::new(0.4, 0.0, 0.0, 0.0);
        let effect = Particle2dEffect {
            color_curve: Some(curve),
            color_spread: Some((spread, spread)),
            ..Default::default()
        };
        let mut store = ParticleStore::default();
        for _ in 0..200 {
            create_particle(&mut store, &effect, &Transform::default());
        }
        update_particles(&mut store, &effect, 0.0, Vec3::ZERO, Vec3::ZERO);
        assert!(store.color_r.iter().all(|r| (0.0..=1.0).contains(r)));
        let (min, max) = store.color_r.iter().fold((1.0_f32, 0.0_f32), |(lo, hi), r| (lo.min(*r), hi.max(*r)));
        assert!(max - min > 0.3, "red varies per particle: {min}..{max}");
        assert!(store.color_g.iter().all(|g| *g == 0.0), "channels without a spread stay put");
    }

    /// cocos2d radius mode `update:`: `angle += dps * dt; radius += deltaRadius * dt; pos = -(cos angle, sin angle) * radius`.
    #[test]
    fn orbit_follows_cocos2d_radius_mode() {
        let origin = Vec2::new(10.0, 20.0);
        let effect = Particle2dEffect {
            orbit: Some(crate::Orbit {
                start_radius: Spread(100.0, 0.0),
                end_radius: Spread(0.0, 0.0),
                turn_rate: Spread(1.5, 0.0),
            }),
            lifetime: Rval::new(0.4, 0.0),
            direction: Some(Rval::new(Vec2::new(0.0, 1.0), 0.0)), // start angle 90 degrees
            linear_speed: None,
            ..Default::default()
        };
        let mut store = ParticleStore::default();
        create_particle(&mut store, &effect, &Transform::from_translation(origin.extend(0.0)));
        let (mut angle, mut radius) = (std::f32::consts::FRAC_PI_2, 100.0_f32);
        let (delta_radius, turn) = (-100.0 / 0.4, 1.5);
        let dt = 1.0 / 30.0;
        for _ in 0..6 {
            update_particles(&mut store, &effect, dt, Vec3::ZERO, Vec3::ZERO);
            angle += turn * dt;
            radius += delta_radius * dt;
            let expected = origin - Vec2::new(angle.cos(), angle.sin()) * radius;
            let actual = Vec2::new(store.position_x[0], store.position_y[0]);
            assert!(actual.distance(expected) < 1e-3, "{actual} != {expected}");
        }
        assert!(radius < 100.0 && radius > 0.0);
    }

    #[test]
    fn rotation_over_life_runs_from_a_random_start_to_a_random_end() {
        let effect = Particle2dEffect {
            rotation_over_life: Some((Spread(1.0, 0.5), Spread(-2.0, 0.0))),
            ..Default::default()
        };
        let mut store = ParticleStore::default();
        for _ in 0..200 {
            create_particle(&mut store, &effect, &Transform::default());
        }
        assert!(store.rotation_start.iter().all(|r| (0.5..=1.5).contains(r)));
        assert!(store.rotation_start.iter().any(|r| *r < 0.8) && store.rotation_start.iter().any(|r| *r > 1.2));
        update_particles(&mut store, &effect, 0.0, Vec3::ZERO, Vec3::ZERO);
        for i in 0..store.len() {
            assert_eq!(store.rotation[i], store.rotation_start[i]);
        }
        store.duration_fraction.iter_mut().for_each(|f| *f = 0.5);
        update_particles(&mut store, &effect, 0.0, Vec3::ZERO, Vec3::ZERO);
        for i in 0..store.len() {
            assert!((store.rotation[i] - (store.rotation_start[i] - 2.0) / 2.0).abs() < 1e-4);
        }
    }

    #[test]
    fn rect_emission_covers_the_half_extents() {
        let half = Vec2::new(10.0, 4.0);
        assert_eq!(EmissionShape::rect_offset(half, 0.0, 0.0), Vec2::new(-10.0, -4.0));
        assert_eq!(EmissionShape::rect_offset(half, 1.0, 1.0), Vec2::new(10.0, 4.0));
        assert_eq!(EmissionShape::rect_offset(half, 0.5, 0.5), Vec2::ZERO);

        // Through create_particle: every spawn lands inside the rectangle, and both axes are used.
        let effect = Particle2dEffect {
            emission_shape: EmissionShape::Rect(half),
            ..Default::default()
        };
        let mut store = ParticleStore::default();
        for _ in 0..200 {
            create_particle(&mut store, &effect, &Transform::default());
        }
        assert!(store.position_x.iter().all(|x| x.abs() <= 10.0));
        assert!(store.position_y.iter().all(|y| y.abs() <= 4.0));
        assert!(store.position_x.iter().any(|x| x.abs() > 5.0));
        assert!(store.position_y.iter().any(|y| y.abs() > 2.0));
    }

    #[test]
    fn gravity_is_an_acceleration() {
        let (gravity, dt, steps) = (-300.0_f32, 1.0 / 30.0, 15);
        let (expected_v, expected_y) = cocos_gravity_steps(gravity, dt, steps);
        let effect = Particle2dEffect::default();

        // 9 particles: 8 go through the SIMD path, the last one through the scalar path.
        let mut particles = ParticleStore::default();
        for _ in 0..9 {
            particles.push(
                Transform::default(),
                1000.0,
                Vec3::ZERO,
                0.0,
                LinearRgba::WHITE,
                0.0,
                0.0,
                0.0,
                0.0,
                -gravity,
                Vec3::NEG_Y,
                Vec2::ZERO,
                0.0,
                0.0,
                SpawnExtras::default(),
            );
        }
        for _ in 0..steps {
            update_particles(&mut particles, &effect, dt, Vec3::ZERO, Vec3::ZERO);
        }
        for index in [0, 8] {
            assert!((particles.velocity_y[index] - expected_v).abs() < 1e-2, "{index}");
            assert!((particles.position_y[index] - expected_y).abs() < 1e-2, "{index}");
        }
    }

    #[test]
    #[ignore = "manual performance benchmark"]
    fn bench_update_one_million_particles() {
        const PARTICLES: usize = 1_000_000;
        const WARMUP: usize = 3;
        const SAMPLES: usize = 20;

        let mut particles = particle_store(PARTICLES);
        let effect = Particle2dEffect::default();

        for _ in 0..WARMUP {
            update_particles(
                black_box(&mut particles),
                black_box(&effect),
                black_box(1.0 / 60.0),
                Vec3::ZERO,
                Vec3::ZERO,
            );
        }

        let mut samples = Vec::with_capacity(SAMPLES);
        for _ in 0..SAMPLES {
            let start = Instant::now();
            update_particles(
                black_box(&mut particles),
                black_box(&effect),
                black_box(1.0 / 60.0),
                Vec3::ZERO,
                Vec3::ZERO,
            );
            samples.push(start.elapsed());
        }
        samples.sort_unstable();

        let median = samples[SAMPLES / 2];
        let total: Duration = samples.iter().sum();
        let average = total / SAMPLES as u32;
        let throughput = PARTICLES as f64 / median.as_secs_f64() / 1_000_000.0;

        black_box(&particles);
        println!(
            "1,000,000 particles: median {median:?}, average {average:?}, {throughput:.2} M particles/s"
        );
    }
}
