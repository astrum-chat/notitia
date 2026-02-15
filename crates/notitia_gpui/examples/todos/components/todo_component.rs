use std::sync::Arc;

use gpui::{App, ElementId, Window, div, prelude::*, px, rgb};

use crate::element_id_ext::ElementIdExt;

type ActionFn = Arc<dyn Fn(&mut Window, &mut App) + 'static>;

#[derive(IntoElement)]
pub struct TodoComponent {
    id: ElementId,
    title: String,
    content: String,
    completed: bool,
    on_toggle: ActionFn,
    on_delete: ActionFn,
}

impl TodoComponent {
    pub fn new(
        id: impl Into<ElementId>,
        title: String,
        content: String,
        completed: bool,
        on_toggle: impl Fn(&mut Window, &mut App) + 'static,
        on_delete: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            title,
            content,
            completed,
            on_toggle: Arc::new(on_toggle),
            on_delete: Arc::new(on_delete),
        }
    }
}

impl RenderOnce for TodoComponent {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let on_toggle = self.on_toggle;
        let on_delete = self.on_delete;
        let completed = self.completed;

        let title_color = if completed {
            rgb(0x888888)
        } else {
            rgb(0xffffff)
        };

        div()
            .w_full()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(10.))
            .p(px(12.))
            .bg(rgb(0x2a2a2a))
            .rounded(px(6.))
            // Check button
            .child(
                div()
                    .id(self.id.with_suffix("check-btn"))
                    .flex_shrink_0()
                    .size(px(24.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.))
                    .cursor_pointer()
                    .border_1()
                    .when(completed, |d| {
                        d.bg(rgb(0x2d8a4e))
                            .border_color(rgb(0x2d8a4e))
                            .text_color(rgb(0xffffff))
                            .child("✓")
                    })
                    .when(!completed, |d| {
                        d.border_color(rgb(0x555555))
                            .hover(|s| s.border_color(rgb(0x888888)))
                    })
                    .on_click(move |_, window, cx| on_toggle(window, cx)),
            )
            // Text content
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .overflow_hidden()
                    .child(
                        div()
                            .text_color(title_color)
                            .text_size(px(15.))
                            .child(self.title),
                    )
                    .when(!self.content.is_empty(), |d| {
                        d.child(
                            div()
                                .text_color(rgb(0x999999))
                                .text_size(px(13.))
                                .child(self.content),
                        )
                    }),
            )
            // Delete button
            .child(
                div()
                    .id(self.id.with_suffix("delete-btn"))
                    .flex_shrink_0()
                    .px(px(10.))
                    .py(px(4.))
                    .bg(rgb(0xcc4444))
                    .text_color(rgb(0xffffff))
                    .text_size(px(12.))
                    .rounded(px(4.))
                    .cursor_pointer()
                    .hover(|s| s.bg(rgb(0xaa3333)))
                    .active(|s| s.bg(rgb(0x882222)))
                    .child("Delete")
                    .on_click(move |_, window, cx| on_delete(window, cx)),
            )
    }
}
