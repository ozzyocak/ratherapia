use bevy_ecs::prelude::*;
use rand::Rng;

#[derive(Component)]
struct Particle {
    x: f32,
    y: f32,
    ttl: f32,
    max_ttl: f32,
    color_index: usize,
}

#[derive(Component)]
struct Velocity {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy)]
pub struct ParticleSnapshot {
    pub x: f32,
    pub y: f32,
    pub alpha: f32,
    pub color_index: usize,
}

pub struct ParticleEngine {
    world: World,
}

impl Default for ParticleEngine {
    fn default() -> Self {
        Self {
            world: World::new(),
        }
    }
}

impl ParticleEngine {
    pub fn burst(&mut self, color_index: usize, count: usize) {
        let mut rng = rand::thread_rng();
        for _ in 0..count {
            let angle = rng.gen_range(0.0..std::f32::consts::TAU);
            let speed = rng.gen_range(0.15..0.85);
            let ttl = rng.gen_range(0.65..1.6);
            self.world.spawn((
                Particle {
                    x: 0.5,
                    y: 0.5,
                    ttl,
                    max_ttl: ttl,
                    color_index,
                },
                Velocity {
                    x: angle.cos() * speed,
                    y: angle.sin() * speed * 0.55,
                },
            ));
        }
    }

    pub fn update(&mut self, dt: f32) {
        let mut expired = Vec::new();
        let mut query = self.world.query::<(Entity, &mut Particle, &Velocity)>();

        for (entity, mut particle, velocity) in query.iter_mut(&mut self.world) {
            particle.x += velocity.x * dt;
            particle.y += velocity.y * dt;
            particle.ttl -= dt;

            if particle.ttl <= 0.0
                || particle.x < -0.05
                || particle.x > 1.05
                || particle.y < -0.05
                || particle.y > 1.05
            {
                expired.push(entity);
            }
        }

        for entity in expired {
            let _ = self.world.despawn(entity);
        }
    }

    pub fn snapshot(&mut self) -> Vec<ParticleSnapshot> {
        let mut query = self.world.query::<&Particle>();
        query
            .iter(&self.world)
            .map(|particle| ParticleSnapshot {
                x: particle.x,
                y: particle.y,
                alpha: (particle.ttl / particle.max_ttl).clamp(0.0, 1.0),
                color_index: particle.color_index,
            })
            .collect()
    }
}
