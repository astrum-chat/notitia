use std::sync::Arc;

use gpui::{App, ElementId, Window, div, hsla, prelude::*, px, rgb};
use gpui_primitives::input::{Input, InputState};

use crate::element_id_ext::ElementIdExt;

type OnSubmitFn = Arc<dyn Fn(String, String, &mut Window, &mut App) + 'static>;

#[derive(IntoElement)]
pub struct AddTodoModal {
    id: ElementId,
    on_submit: OnSubmitFn,
}

impl AddTodoModal {
    pub fn new(
        id: impl Into<ElementId>,
        on_submit: impl Fn(String, String, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            on_submit: Arc::new(on_submit),
        }
    }
}

impl RenderOnce for AddTodoModal {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let title_state =
            window.use_keyed_state(self.id.with_suffix("title"), cx, |_window, cx| {
                InputState::new(cx)
            });
        let content_state =
            window.use_keyed_state(self.id.with_suffix("content"), cx, |_window, cx| {
                InputState::new(cx)
            });
        let on_submit = self.on_submit;

        let title_clone = title_state.clone();
        let content_clone = content_state.clone();

        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(8.))
            .p(px(14.))
            .bg(rgb(0x2a2a2a))
            .child(
                div()
                    .text_color(rgb(0xffffff))
                    .text_size(px(16.))
                    .child("Add Todo"),
            )
            .child(
                Input::new(self.id.with_suffix("title-input"), title_state)
                    .placeholder("Title")
                    .placeholder_text_color(hsla(0., 0., 1., 0.35))
                    .bg(rgb(0x3a3a3a))
                    .text_color(rgb(0xffffff))
                    .rounded(px(4.))
                    .px(px(8.))
                    .py(px(6.)),
            )
            .child(
                Input::new(self.id.with_suffix("content-input"), content_state)
                    .placeholder("Description")
                    .placeholder_text_color(hsla(0., 0., 1., 0.35))
                    .bg(rgb(0x3a3a3a))
                    .text_color(rgb(0xffffff))
                    .rounded(px(4.))
                    .px(px(8.))
                    .py(px(6.)),
            )
            .child(
                div()
                    .id(self.id.with_suffix("submit-btn"))
                    .px(px(12.))
                    .py(px(6.))
                    .bg(rgb(0x4488ff))
                    .text_color(rgb(0xffffff))
                    .rounded(px(4.))
                    .cursor_pointer()
                    .hover(|s| s.bg(rgb(0x3377ee)))
                    .active(|s| s.bg(rgb(0x2266dd)))
                    .child("Submit")
                    .on_click(move |_, window, cx| {
                        let title = title_clone.read(cx).value().to_string();
                        let content = content_clone.read(cx).value().to_string();

                        if !title.is_empty() {
                            on_submit(title, content, window, cx);
                        }

                        title_clone.update(cx, |s, _| {
                            s.clear();
                        });
                        content_clone.update(cx, |s, _| {
                            s.clear();
                        });
                    }),
            )
    }
}
