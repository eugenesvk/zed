use std::sync::Arc;

use crate::{EditPredictionStore, ZedPredictUpsell};

use client::{Client, UserStore};
use db::kvp::Dismissable;
use fs::Fs;
use gpui::{
    ClickEvent, DismissEvent, Entity, EventEmitter, FocusHandle, Focusable, MouseDownEvent, Render,
    linear_color_stop, linear_gradient,
};
use language::language_settings::EditPredictionProvider;
use settings::update_settings_file;
use ui::prelude::*;
use workspace::{ModalView, Workspace};

#[macro_export]
macro_rules! onboarding_event {
    ($name:expr) => {
        telemetry::event!($name, source = "Edit Prediction Onboarding");
    };
    ($name:expr, $($key:ident $(= $value:expr)?),+ $(,)?) => {
        telemetry::event!($name, source = "Edit Prediction Onboarding", $($key $(= $value)?),+);
    };
}



pub(crate) fn set_edit_prediction_provider(provider: EditPredictionProvider, cx: &mut App) {
    let fs = <dyn Fs>::global(cx);
    update_settings_file(fs, cx, move |settings, _| {
        settings
            .project
            .all_languages
            .edit_predictions
            .get_or_insert(Default::default())
            .provider = Some(provider);
    });
}










