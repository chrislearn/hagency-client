use makepad_widgets::*;

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    // Legacy names share the same flat controls as new screens.
    mod.widgets.RobrixIconButton = mod.widgets.RinxPrimaryButton {text: ""}
    mod.widgets.RobrixNeutralIconButton = mod.widgets.RinxButton {text: ""}
    mod.widgets.RobrixPositiveIconButton = mod.widgets.RinxButton {
        text: ""
        draw_bg +: {color: RINX_SUCCESS_BG color_focus: RINX_SUCCESS_BG color_hover: RINX_HOVER color_down: RINX_PRESSED}
        draw_text +: {color: RINX_SUCCESS_FG color_hover: RINX_SUCCESS_FG color_down: RINX_SUCCESS_FG color_focus: RINX_SUCCESS_FG}
        draw_icon.color: RINX_SUCCESS_FG
    }
    mod.widgets.RobrixNegativeIconButton = mod.widgets.RinxButton {
        text: ""
        draw_bg +: {color: RINX_DANGER_BG color_focus: RINX_DANGER_BG color_hover: RINX_HOVER color_down: RINX_PRESSED}
        draw_text +: {color: RINX_DANGER_FG color_hover: RINX_DANGER_FG color_down: RINX_DANGER_FG color_focus: RINX_DANGER_FG}
        draw_icon.color: RINX_DANGER_FG
    }
    mod.widgets.RinxBackButton = mod.widgets.RinxGhostButton {
        text: #(crate::i18n::tr("Back")) i18n_text: "Back"
        padding: Inset{left: 8 right: 8 top: 0 bottom: 0}
        draw_icon.svg: ICON_ARROW_BACK
    }

    mod.widgets.RinxChatInfoButton = mod.widgets.RobrixNeutralIconButton {
        width: RINX_ICON_BUTTON_SIZE height: RINX_ICON_BUTTON_SIZE padding: 0 spacing: 0
        align: Align{x: 0.5 y: 0.5}
        text: "···"
        draw_text +: {color: mod.widgets.RINX_INK text_style: theme.font_bold {font_size: RINX_HEADING_SIZE}}
        draw_bg +: {color: #0000 color_hover: mod.widgets.RINX_HOVER color_down: mod.widgets.RINX_PRESSED border_size: 0}
        icon_walk: Walk{width: 0 height: 0}
    }

    mod.widgets.RinxRoomInfoToggleButton = mod.widgets.NavigationBarButton {
        tooltip_below: true
        width: RINX_ICON_BUTTON_SIZE height: RINX_ICON_BUTTON_SIZE padding: 0 spacing: 0
        align: Align{x: 0.5 y: 0.5}
        tooltip_text: #(crate::i18n::tr("Show room information"))
        icon := Icon {icon_walk: Walk{width: 18 height: 18} draw_icon +: {svg: ICON_PANEL_RIGHT color: mod.widgets.RINX_INK}}
        draw_bg +: {color_hover: mod.widgets.RINX_HOVER color_active: mod.widgets.RINX_SELECTED border_size: 0}
    }
}
