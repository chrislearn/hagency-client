use makepad_widgets::*;
use super::{KanbanCard, ProjectBoard};

makepad_widgets::script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*
    let Text = RinxLabel {width: Fill height: Fit draw_text +: {color: mod.widgets.RINX_INK text_style: theme.font_regular{font_size: mod.widgets.RINX_BODY_SIZE}}}
    let Small = Text {draw_text +: {color: mod.widgets.RINX_MUTED text_style: theme.font_regular{font_size: mod.widgets.RINX_META_SIZE}}}
    let Input = RobrixTextInput {width: Fill height: 36 margin: 0 padding: Inset{left: 10 right: 10 top: 8 bottom: 8}}
    let Select = RobrixSettingsDropDown {width: Fill height: 36 margin: 0}
    let Action = RinxButton {height: 34 padding: Inset{left: 12 right: 12 top: 0 bottom: 0}}
    mod.widgets.KanbanCard = #(KanbanCard::register_widget(vm)) {
        width: Fill height: Fit margin: Inset{bottom: 8} padding: 12 flow: Down spacing: 9
        show_bg: true cursor: MouseCursor.Hand
        draw_bg +: {color: mod.widgets.RINX_SURFACE border_radius: 8.0 border_size: 1.0 border_color: mod.widgets.RINX_BORDER}
        View {width: Fill height: Fit align: Align{y: 0.5}
            number := Small {width: Fill}
            priority := Small {width: Fit}
            drag_handle := View {width: 26 height: 24 align: Align{x: 1.0 y: 0.5} cursor: MouseCursor.Move
                Small {width: Fit text: "⠿"}
            }
        }
        title := Text {max_lines: 3 text_overflow: Ellipsis draw_text.text_style: theme.font_bold{font_size: mod.widgets.RINX_BODY_SIZE}}
        tags := Small {max_lines: 1 text_overflow: Ellipsis}
        View {width: Fill height: Fit spacing: 8
            assignee := Small {width: Fill max_lines: 1 text_overflow: Ellipsis}
            due := Small {width: Fit}
        }
    }
    let Column = RoundedView {width: 260 height: Fill padding: 10 flow: Down spacing: 10
        draw_bg +: {color: mod.widgets.RINX_PANEL_BG border_radius: 10.0}
        View {width: Fill height: 28 spacing: 8 align: Align{y: 0.5}
            marker := RoundedView {width: 7 height: 7 draw_bg +: {color: mod.widgets.RINX_MUTED border_radius: 3.5}}
            heading := Text {width: Fill draw_text.text_style: theme.font_bold{font_size: mod.widgets.RINX_BODY_SIZE}}
            count := Small {width: Fit}
        }
        cards := PortalList {width: Fill height: Fill capture_overload: false
            Card := mod.widgets.KanbanCard {}
            Empty := View {width: Fill height: 92 padding: 14
                Small {text: #(crate::i18n::tr("No tasks here yet")) i18n_text: "No tasks here yet"}
            }
            More := View {width: Fill height: 48
                more := Action {width: Fill text: #(crate::i18n::tr("Load more tasks")) i18n_text: "Load more tasks"}
            }
            Filler := View {width: Fill height: 12}
        }
    }
    mod.widgets.ProjectBoard = #(ProjectBoard::register_widget(vm)) {
        skip_widget_tree_search: true
        width: Fill height: Fill flow: Down spacing: 12
        View {width: Fill height: Fit flow: Flow.Right{wrap: true} spacing: 10 align: Align{y: 0.5}
            search := Input {width: 220 empty_text: #(crate::i18n::tr("Search tasks…")) i18n_empty_text: "Search tasks…"}
            filter_state := Select {width: 150 labels: [#(crate::i18n::tr("All statuses")) #(crate::i18n::tr("To do")) #(crate::i18n::tr("In progress")) #(crate::i18n::tr("Blocked")) #(crate::i18n::tr("In review")) #(crate::i18n::tr("Done"))]}
            filter_assignee := Select {width: 180 labels: [#(crate::i18n::tr("All members"))]}
            filter_label := Input {width: 120 empty_text: #(crate::i18n::tr("Label")) i18n_empty_text: "Label"}
            apply_filters := Action {text: #(crate::i18n::tr("Filter")) i18n_text: "Filter"}
            refresh := Action {text: #(crate::i18n::tr("Refresh")) i18n_text: "Refresh"}
            new_task := RinxPrimaryButton {height: 34 text: #(crate::i18n::tr("New task")) i18n_text: "New task"}
        }
        View {width: Fill height: Fit flow: Flow.Right{wrap: true} spacing: 14 align: Align{y: 0.5}
            summary := Small {width: Fit text: #(crate::i18n::tr("Shared with Project members")) i18n_text: "Shared with Project members"}
            overdue := RinxCheckBox {text: #(crate::i18n::tr("Overdue")) i18n_text: "Overdue"}
            archived := RinxCheckBox {text: #(crate::i18n::tr("Archived tasks")) i18n_text: "Archived tasks"}
            sync_status := Small {width: Fit}
        }
        notice := Text {visible: false draw_text.color: mod.widgets.RINX_MUTED}
        retry := Action {visible: false text: #(crate::i18n::tr("Retry original save")) i18n_text: "Retry original save"}
        workspace := View {width: Fill height: Fill spacing: 14
            board_scroll := ScrollXView {width: Fill height: Fill flow: Right spacing: 12
                todo := Column {}
                in_progress := Column {}
                blocked := Column {}
                in_review := Column {}
                done := Column {}
            }
            inspector := RoundedView {visible: false width: 340 height: Fill padding: 16 flow: Down spacing: 10
                draw_bg +: {color: mod.widgets.RINX_SURFACE border_radius: 10.0 border_size: 1.0 border_color: mod.widgets.RINX_BORDER}
                View {width: Fill height: Fit align: Align{y: 0.5}
                    detail_heading := Text {width: Fill draw_text.text_style: theme.font_bold{font_size: mod.widgets.RINX_BODY_SIZE}}
                    close_detail := Action {text: #(crate::i18n::tr("Close")) i18n_text: "Close"}
                }
                editor_scroll := ScrollYView {width: Fill height: Fill flow: Down spacing: 10
                    detail_notice := Small {}
                    Text {text: #(crate::i18n::tr("Title")) i18n_text: "Title"}
                    title := Input {empty_text: #(crate::i18n::tr("What needs to be done?")) i18n_empty_text: "What needs to be done?"}
                    Text {text: #(crate::i18n::tr("Description")) i18n_text: "Description"}
                    description := Input {height: 130 is_multiline: true empty_text: #(crate::i18n::tr("Add context and acceptance criteria…")) i18n_empty_text: "Add context and acceptance criteria…"}
                    View {width: Fill height: Fit flow: Down spacing: 5
                        Text {text: #(crate::i18n::tr("Priority")) i18n_text: "Priority"}
                        priority := Select {labels: [#(crate::i18n::tr("Low")) #(crate::i18n::tr("Normal")) #(crate::i18n::tr("High")) #(crate::i18n::tr("Urgent"))]}
                    }
                    Text {text: #(crate::i18n::tr("Assignee")) i18n_text: "Assignee"}
                    assignee := Select {labels: [#(crate::i18n::tr("Unassigned"))]}
                    Text {text: #(crate::i18n::tr("Labels")) i18n_text: "Labels"}
                    labels := Input {empty_text: #(crate::i18n::tr("Comma-separated labels")) i18n_empty_text: "Comma-separated labels"}
                    Text {text: #(crate::i18n::tr("Due date")) i18n_text: "Due date"}
                    due_date := Input {empty_text: "YYYY-MM-DD"}
                    save := RinxPrimaryButton {width: Fill height: 36 text: #(crate::i18n::tr("Save task")) i18n_text: "Save task"}
                    reload_latest := Action {visible: false width: Fill text: #(crate::i18n::tr("Discard draft and load latest")) i18n_text: "Discard draft and load latest"}
                    existing_actions := View {visible: false width: Fill height: Fit flow: Down spacing: 8
                        Hr {width: Fill height: 1}
                        Text {text: #(crate::i18n::tr("Move task")) i18n_text: "Move task"}
                        move_state := Select {labels: [#(crate::i18n::tr("To do")) #(crate::i18n::tr("In progress")) #(crate::i18n::tr("Blocked")) #(crate::i18n::tr("In review")) #(crate::i18n::tr("Done"))]}
                        View {width: Fill height: Fit spacing: 8
                            move_top := Action {text: #(crate::i18n::tr("Move to top")) i18n_text: "Move to top"}
                            move_bottom := Action {text: #(crate::i18n::tr("Move to bottom")) i18n_text: "Move to bottom"}
                        }
                        archive := Action {width: Fill text: #(crate::i18n::tr("Archive task")) i18n_text: "Archive task"}
                        Hr {width: Fill height: 1}
                        Text {text: #(crate::i18n::tr("Comments")) i18n_text: "Comments"}
                        comments := Text {}
                        more_comments := Action {visible: false text: #(crate::i18n::tr("Load more comments")) i18n_text: "Load more comments"}
                        first_comments := Action {visible:false text:#(crate::i18n::tr("Back to first comments")) i18n_text:"Back to first comments"}
                        comment := Input {height: 76 is_multiline: true empty_text: #(crate::i18n::tr("Write a comment…")) i18n_empty_text: "Write a comment…"}
                        add_comment := Action {text: #(crate::i18n::tr("Add comment")) i18n_text: "Add comment"}
                        Text {text: #(crate::i18n::tr("Activity")) i18n_text: "Activity"}
                        activity := Small {}
                        more_activity := Action {visible: false text: #(crate::i18n::tr("Load more activity")) i18n_text: "Load more activity"}
                        first_activity := Action {visible:false text:#(crate::i18n::tr("Back to first activity")) i18n_text:"Back to first activity"}
                    }
                }
            }
        }
        drag_notice := Small {visible: false text: #(crate::i18n::tr("Drop on a column to move. Use the task menu for precise placement.")) i18n_text: "Drop on a column to move. Use the task menu for precise placement."}
        drag_preview := Tooltip {abs_pos:vec2(0,0) content +: {padding:12 draw_bg +: {color:mod.widgets.RINX_SURFACE border_color:mod.widgets.RINX_ACCENT border_radius:8.0}
            tooltip_label +: {width:230 max_lines:2 text_overflow:Ellipsis draw_text +: {color:mod.widgets.RINX_INK text_style:theme.font_regular{font_size:mod.widgets.RINX_BODY_SIZE}}}
        }}
    }
}
