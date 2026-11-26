use super::*;
use crate::hagency::project_order::{self, OrderKind, ProjectOrderChanged};

impl HagencyPanel {
    fn selected_project_space(&self, cx: &mut Cx) -> Option<(String, String)> {
        let project = self.selected(cx, ids!(projects), &self.projects)?;
        Some((
            project["id"].as_str()?.into(),
            project["spaceId"].as_str()?.into(),
        ))
    }

    pub(super) fn refresh_project_order(&self, cx: &mut Cx) {
        if let Some((project, space)) = self.selected_project_space(cx) {
            project_order::refresh(cx, &self.identity, &project, &space);
        }
    }

    pub(super) fn retry_project_order(&self, cx: &mut Cx) {
        if let Some((project, space)) = self.selected_project_space(cx) {
            project_order::fetch(cx, &self.identity, &project, &space, true);
        }
    }

    pub(super) fn save_project_order(&mut self, cx: &mut Cx, kind: OrderKind, ids: Vec<String>) {
        let Some((project, space)) = self.selected_project_space(cx) else {
            return;
        };
        if self.project_home["projectId"] != project
            || self.project_home["canManagePolicy"] != true
            || self.project_home["archived"] == true
        {
            return;
        }
        if project_order::save(cx, &self.identity, &project, &space, kind, ids) {
            self.status(cx, "");
        } else {
            self.status(
                cx,
                crate::i18n::tr("Could not save the project order. Try again."),
            );
        }
        self.view.redraw(cx);
    }

    pub(super) fn on_project_order_changed(&mut self, cx: &mut Cx, changed: &ProjectOrderChanged) {
        if self.identity != changed.identity
            || self.project(cx).as_deref() != Some(&changed.project)
        {
            return;
        }
        self.apply_project_order(cx);
        self.resume_project_order_navigation(cx);
        if let Some(error) = changed.error {
            self.status(cx, crate::i18n::tr(error));
        }
    }
}
