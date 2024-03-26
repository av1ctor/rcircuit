use rcircuit_engine::entities::EntityState;
use neuralarena_cdk::models::{
    renderer::{ModelRef, Renderer},
    assets::{Asset, AssetRef}
};
use three_rs::math::{
    Vector3, Quaternion
};

struct Object {
}

pub struct NullRenderer {
    objects: Vec<Object>,
}

impl NullRenderer {
    pub fn new(
    ) -> Self {
        Self {
            objects: vec![],
        }
    }
}

impl Renderer<EntityState> for NullRenderer {
    fn reset(
        &mut self
    ) {
    }

    fn load_asset(
        &mut self,
        _asset_ref: AssetRef,
        _asset: &Asset
    ) -> Result<(), String> {
        Ok(())
    }

    fn add_model(
        &mut self,
        _asset_ref: AssetRef
    ) -> Result<ModelRef, String> {
        let model_ref = self.objects.len();
        self.objects.push(Object{});
        Ok(model_ref)
    }

    fn show_model(
        &mut self,
        _model_ref: ModelRef
    ) {
    }

    fn hide_model(
        &mut self,
        _model_ref: ModelRef
    ) {
    }

    fn set_model_position(
        &mut self,
        _model_ref: ModelRef,
        _pos: &Vector3
    ) {
    }

    fn set_model_rotation(
        &mut self,
        _model_ref: ModelRef,
        _q: &Quaternion
    ) {
    }

    fn set_model_scale(
        &mut self,
        _model_ref: ModelRef,
        _scale: &Vector3
    ) {
    }

    fn change_model_state(
        &mut self,
        _model_ref: ModelRef,
        _from: EntityState,
        _to: EntityState
    ) {
    }

    fn update(
        &mut self,
        _delta_time: f32
    ) {
    }

    fn render(
        &mut self
    ) {
    }
}