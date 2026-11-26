//! Shared settings dropdown face and popup styles.
use makepad_widgets::*;

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    // A single item within a Rinx-styled settings DropDown popup menu.
    mod.widgets.RobrixSettingsPopupMenuItem = PopupMenuItem {
        width: Fill, height: RINX_BUTTON_HEIGHT
        align: Align{y: 0.5}
        padding: Inset{top: 0, bottom: 0, left: 28, right: 12}

        draw_text +: {
            color: (MESSAGE_TEXT_COLOR),
            color_hover: (MESSAGE_TEXT_COLOR),
            color_active: RINX_ACCENT,
            text_style: SETTINGS_REGULAR_TEXT_STYLE {},
        }

        draw_bg +: {
            color: (COLOR_PRIMARY),
            color_hover: RINX_HOVER,
            color_active: (COLOR_BG_PREVIEW),
            border_color: vec4(0.0, 0.0, 0.0, 0.0),
            border_color_hover: vec4(0.0, 0.0, 0.0, 0.0),
            border_color_active: vec4(0.0, 0.0, 0.0, 0.0),
            border_size: 0.0,
            border_radius: RINX_CONTROL_RADIUS,
            mark_color: vec4(0.0, 0.0, 0.0, 0.0),
            mark_color_active: RINX_ACCENT,
        }
    }

    // The popup list shown when a RobrixSettingsDropDown is opened.
    mod.widgets.RobrixSettingsPopupMenu = PopupMenu {
        width: 218, height: Fit
        padding: 4,

        menu_item: mod.widgets.RobrixSettingsPopupMenuItem{}

        draw_bg +: {
            color: (COLOR_PRIMARY),
            border_color: (COLOR_SECONDARY_DARKER),
            border_size: 1.0,
            border_radius: RINX_CONTROL_RADIUS,
        }
    }

    // A DropDown styled to match other Rinx settings controls.
    mod.widgets.RinxDropDown = DropDownFlat {
        width: 218, height: mod.widgets.RINX_CONTROL_HEIGHT,
        padding: Inset{top: 0, bottom: 0, left: 10, right: 30}
        margin: 0
        align: Align{x: 0.0, y: 0.5}

        popup_menu: mod.widgets.RobrixSettingsPopupMenu {}

        draw_text +: {
            color: (MESSAGE_TEXT_COLOR),
            color_hover: (MESSAGE_TEXT_COLOR),
            color_focus: (MESSAGE_TEXT_COLOR),
            color_down: (MESSAGE_TEXT_COLOR),
            color_disabled: RINX_DISABLED,
            text_style: SETTINGS_REGULAR_TEXT_STYLE {},
        }

        draw_bg +: {
            color: (COLOR_PRIMARY),
            color_hover: RINX_HOVER,
            color_down: RINX_PRESSED,
            color_focus: RINX_FIELD,
            color_disabled: RINX_FIELD,
            border_color: RINX_CONTROL_BORDER,
            border_color_disabled: RINX_BORDER,
            border_color_hover: (COLOR_ACTIVE_PRIMARY),
            border_color_focus: RINX_ACCENT,
            border_color_down: (COLOR_ACTIVE_PRIMARY_DARKER),
            border_size: 1.0,
            border_radius: RINX_CONTROL_RADIUS,
            arrow_color: RINX_MUTED,
            arrow_color_disabled: RINX_DISABLED,
            arrow_color_hover: (COLOR_ACTIVE_PRIMARY_DARKER),
            arrow_color_focus: (COLOR_ACTIVE_PRIMARY_DARKER),
            arrow_color_down: (COLOR_ACTIVE_PRIMARY_DARKER),

            // The base DropDownFlat shader draws the arrow BEFORE the box,
            // so the box fill paints over it. Override to draw the rounded
            // rect first and then the arrow on top.
            pixel: fn() {
                let sdf = Sdf2d.viewport(self.pos * self.rect_size)

                let line = max(self.border_size, 2.0 * self.focus * (1.0 - self.disabled))
                let inset = line * 0.5
                sdf.box(
                    inset
                    inset
                    self.rect_size.x - line
                    self.rect_size.y - line
                    self.border_radius
                )

                let fill = self.color
                    .mix(self.color_focus, self.focus)
                    .mix(self.color_hover, self.hover)
                    .mix(self.color_down, self.down * self.hover)
                    .mix(self.color_disabled, self.disabled)

                let stroke = self.border_color
                    .mix(self.border_color_hover, self.hover)
                    .mix(self.border_color_down, self.down)
                    .mix(self.border_color_focus, self.focus)
                    .mix(self.border_color_disabled, self.disabled)

                sdf.fill_keep(fill)
                sdf.stroke(stroke, line)

                // Draw the down-arrow triangle on top of the filled box.
                let c = vec2(self.rect_size.x - 14.0, self.rect_size.y * 0.5)
                let sz = 3.5
                sdf.move_to(c.x - sz, c.y - sz * 0.5)
                sdf.line_to(c.x + sz, c.y - sz * 0.5)
                sdf.line_to(c.x, c.y + sz * 0.75)
                sdf.close_path()

                let arrow = self.arrow_color
                    .mix(self.arrow_color_focus, self.focus)
                    .mix(self.arrow_color_hover, self.hover)
                    .mix(self.arrow_color_down, self.down * self.hover)
                    .mix(self.arrow_color_disabled, self.disabled)

                sdf.fill(arrow)

                return sdf.result
            }
        }
    }

    mod.widgets.RobrixSettingsDropDown = mod.widgets.RinxDropDown {}
}
