mod edit_prediction_button;
mod edit_prediction_context_view;


use command_palette_hooks::CommandPaletteFilter;
use edit_prediction::ResetOnboarding;
use edit_prediction_context_view::EditPredictionContextView;
use feature_flags::FeatureFlagAppExt as _;
use gpui::actions;
use project::DisableAiSettings;

use settings::{Settings as _, SettingsStore};
use std::any::{Any as _, TypeId};
use ui::{App, prelude::*};
use workspace::{SplitDirection, Workspace};

pub use edit_prediction_button::{
    EditPredictionButton, ToggleMenu, get_available_providers, set_completion_provider,
};



actions!(
    dev,
    [
        /// Opens the edit prediction context view.
        OpenEditPredictionContextView,
    ]
);

actions!(
    edit_prediction,
    [
        /// Opens the rate completions modal.
        RatePredictions,
    ]
);

pub fn init(cx: &mut App) {
    

    cx.observe_new(move |workspace: &mut Workspace, _, _cx| {
        

        workspace.register_action_renderer(|div, _, _, cx| {
            div.on_action(cx.listener(
                move |workspace, _: &OpenEditPredictionContextView, window, cx| {
                    let project = workspace.project();
                    workspace.split_item(
                        SplitDirection::Right,
                        Box::new(cx.new(|cx| {
                            EditPredictionContextView::new(
                                project.clone(),
                                workspace.client(),
                                workspace.user_store(),
                                window,
                                cx,
                            )
                        })),
                        window,
                        cx,
                    );
                },
            ))
        });
    })
    .detach();
}


