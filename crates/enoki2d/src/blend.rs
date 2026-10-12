use bevy_reflect::Reflect;
use bevy_render::render_resource::{
    BlendComponent, BlendFactor as WgpuBlendFactor, BlendOperation, BlendState,
};
use serde::{Deserialize, Serialize};

/// One side of a blend equation. Mirrors the classic GL blend factors.
#[derive(Deserialize, Serialize, Reflect, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlendFactor {
    Zero,
    One,
    Src,
    OneMinusSrc,
    SrcAlpha,
    OneMinusSrcAlpha,
    Dst,
    OneMinusDst,
    DstAlpha,
    OneMinusDstAlpha,
}

impl From<BlendFactor> for WgpuBlendFactor {
    fn from(value: BlendFactor) -> Self {
        match value {
            BlendFactor::Zero => WgpuBlendFactor::Zero,
            BlendFactor::One => WgpuBlendFactor::One,
            BlendFactor::Src => WgpuBlendFactor::Src,
            BlendFactor::OneMinusSrc => WgpuBlendFactor::OneMinusSrc,
            BlendFactor::SrcAlpha => WgpuBlendFactor::SrcAlpha,
            BlendFactor::OneMinusSrcAlpha => WgpuBlendFactor::OneMinusSrcAlpha,
            BlendFactor::Dst => WgpuBlendFactor::Dst,
            BlendFactor::OneMinusDst => WgpuBlendFactor::OneMinusDst,
            BlendFactor::DstAlpha => WgpuBlendFactor::DstAlpha,
            BlendFactor::OneMinusDstAlpha => WgpuBlendFactor::OneMinusDstAlpha,
        }
    }
}

/// How the particles of an effect are blended into the target.
#[derive(Deserialize, Serialize, Reflect, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ParticleBlend {
    /// Standard alpha blending (`SrcAlpha`, `OneMinusSrcAlpha`).
    #[default]
    Alpha,
    /// Additive blending (`SrcAlpha`, `One`).
    Additive,
    /// A factor pair, applied to colour and alpha alike (like `glBlendFunc`).
    Custom(BlendFactor, BlendFactor),
}

impl ParticleBlend {
    pub(crate) fn blend_state(self) -> BlendState {
        let pair = |src: BlendFactor, dst: BlendFactor| {
            let component = BlendComponent {
                src_factor: src.into(),
                dst_factor: dst.into(),
                operation: BlendOperation::Add,
            };
            BlendState {
                color: component,
                alpha: component,
            }
        };
        match self {
            ParticleBlend::Alpha => BlendState::ALPHA_BLENDING,
            ParticleBlend::Additive => pair(BlendFactor::SrcAlpha, BlendFactor::One),
            ParticleBlend::Custom(src, dst) => pair(src, dst),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::Particle2dPipelineKey;
    use bevy_sprite_render::Mesh2dPipelineKey;

    #[test]
    fn alpha_is_the_default_and_keeps_the_old_state() {
        assert_eq!(ParticleBlend::default(), ParticleBlend::Alpha);
        assert_eq!(ParticleBlend::Alpha.blend_state(), BlendState::ALPHA_BLENDING);
    }

    #[test]
    fn custom_maps_factors_for_colour_and_alpha() {
        let state = ParticleBlend::Custom(BlendFactor::One, BlendFactor::DstAlpha).blend_state();
        assert_eq!(state.color.src_factor, WgpuBlendFactor::One);
        assert_eq!(state.color.dst_factor, WgpuBlendFactor::DstAlpha);
        assert_eq!(state.alpha, state.color);
        let additive = ParticleBlend::Additive.blend_state();
        assert_eq!(additive.color.src_factor, WgpuBlendFactor::SrcAlpha);
        assert_eq!(additive.color.dst_factor, WgpuBlendFactor::One);
    }

    #[test]
    fn pipeline_key_differs_per_blend() {
        let mesh_key = Mesh2dPipelineKey::from_msaa_samples(1);
        let key = |blend| Particle2dPipelineKey::new(mesh_key, blend);
        assert_eq!(key(ParticleBlend::Alpha), key(ParticleBlend::Alpha));
        assert_ne!(key(ParticleBlend::Alpha), key(ParticleBlend::Additive));
        assert_ne!(
            key(ParticleBlend::Custom(BlendFactor::One, BlendFactor::One)),
            key(ParticleBlend::Custom(BlendFactor::One, BlendFactor::DstAlpha))
        );
    }

    #[test]
    fn effect_round_trips_the_blend_field() {
        let effect = crate::Particle2dEffect {
            blend: ParticleBlend::Custom(BlendFactor::One, BlendFactor::DstAlpha),
            ..Default::default()
        };
        let text = ron::to_string(&effect).unwrap();
        let back: crate::Particle2dEffect = ron::from_str(&text).unwrap();
        assert_eq!(back.blend, effect.blend);
    }

    #[test]
    fn effect_without_a_blend_field_loads_as_alpha() {
        let text = ron::to_string(&crate::Particle2dEffect::default()).unwrap();
        assert!(text.contains("blend:Alpha"), "{text}");
        let old = text.replace(",blend:Alpha", "");
        assert!(!old.contains("blend"));
        let back: crate::Particle2dEffect = ron::from_str(&old).unwrap();
        assert_eq!(back.blend, ParticleBlend::Alpha);
    }
}
