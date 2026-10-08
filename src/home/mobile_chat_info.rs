//! Mobile room information backed by the current Matrix room and push rules.

use makepad_widgets::*;
use std::collections::HashMap;
use matrix_sdk::{
    notification_settings::RoomNotificationMode,
    ruma::{OwnedUserId, OwnedRoomId},
    RoomMemberships,
};

use crate::{
    home::{invite_modal::InviteModalAction, rooms_list::{RoomNotificationModeUpdated, RoomsListRef}},
    logout::logout_confirm_modal::LogoutAction,
    profile::user_profile::{ShowUserProfileAction, UserProfile, UserProfileAndRoomId},
    shared::{
        avatar::{AvatarState, AvatarWidgetRefExt},
        navigation_bar_button::{NavigationBarButtonWidgetRefExt, NavigationBarButtonWidgetExt},
    },
    sliding_sync::{
        current_user_id, get_client, spawn_async_task, submit_async_request, MatrixRequest,
    },
    utils::{self, RoomNameId},
};

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    mod.widgets.MobileChatInfo = #(MobileChatInfo::register_widget(vm)) {
        ..mod.widgets.SolidView
        width: Fill height: Fill flow: Down
        draw_bg.color: mod.widgets.MOBILE_BG
        summary := View {width: Fill height: Fit flow: Down padding: 16 spacing: 8
            summary_name := Label {width: Fill height: Fit max_lines: 2 text_overflow: Ellipsis
                draw_text +: {color: mod.widgets.RINX_INK text_style: theme.font_bold{font_size: (15 * mod.widgets.RINX_TEXT_SCALE)}}}
            View {width: Fill height: 32 flow: Right spacing: 8 align: Align{y: 0.5}
                summary_count := Label {width: Fill height: Fit draw_text +: {color: mod.widgets.RINX_MUTED text_style: theme.font_regular{font_size: (11 * mod.widgets.RINX_TEXT_SCALE)}}}
                history_search := NavigationBarButton {width: 32 height: 32 padding: 7 tooltip_text: #(crate::i18n::tr("Search Chat History"))
                    Icon {icon_walk: Walk{width: 16 height: 16} draw_icon +: {svg: ICON_SEARCH color: mod.widgets.RINX_MUTED}}}
                history_media := NavigationBarButton {width: 32 height: 32 padding: 7 tooltip_text: #(crate::i18n::tr("Shared Attachments"))
                    Icon {icon_walk: Walk{width: 16 height: 16} draw_icon +: {svg: ICON_ADD_ATTACHMENT color: mod.widgets.RINX_MUTED}}}
            }
            View {width: Fill height: 36 flow: Right spacing: 8
                overview_tab := RobrixNeutralIconButton {width: Fill height: 36 text: #(crate::i18n::tr("Overview")) icon_walk: Walk{width: 0 height: 0}}
                members_tab := RobrixNeutralIconButton {width: Fill height: 36 text: #(crate::i18n::tr("Members")) icon_walk: Walk{width: 0 height: 0}}
            }
        }
        status := Label {
            width: Fill height: Fit margin: 16
            flow: Flow.Right{wrap: true}
            draw_text +: {color: mod.widgets.RINX_MUTED text_style: theme.font_regular {font_size: (11 * mod.widgets.RINX_TEXT_SCALE)}}
        }
        retry := RobrixNeutralIconButton {visible: false text: #(crate::i18n::tr("Retry")) i18n_text: "Retry" height: 44 margin: 12}
        services_panel := View {visible: false width: Fill height: Fit flow: Down padding: Inset{left: 16 right: 16 bottom: 12} spacing: 8
            services_status := Label {width: Fill height: Fit flow: Flow.Right{wrap: true} draw_text.color: mod.widgets.RINX_MUTED}
            room_services := RobrixSettingsDropDown {width: Fill height: 40 margin: 0 labels: ["Choose an Agent"]
                popup_menu_position: PopupMenuPosition.BelowInput
                popup_menu: RobrixSettingsPopupMenu {width: 256 menu_item: RobrixSettingsPopupMenuItem {height: 36}}}
            View {width: Fill height: 36 flow: Right spacing: 8
                configure_service := RobrixNeutralIconButton {text: #(crate::i18n::tr("Service settings")) height: 36 width: Fill icon_walk: Walk{width: 0 height: 0}}
                refresh_services := RobrixNeutralIconButton {text: #(crate::i18n::tr("Refresh")) height: 36 icon_walk: Walk{width: 0 height: 0}}
            }
        }
        member_controls := View {visible: false width: Fill height: Fit flow: Down padding: Inset{left: 12 right: 12 bottom: 12} spacing: 8
            View {width: Fill height: 32 flow: Right spacing: 6
                invite_people := RobrixNeutralIconButton {text: #(crate::i18n::tr("Invite people")) i18n_text: "Invite people" width: Fill height: 32 padding: 6 spacing: 5
                    icon_walk: Walk{width: 14 height: 14} draw_icon.svg: ICON_ADD_USER}
                invite_my_agent := RobrixNeutralIconButton {text: #(crate::i18n::tr("Add agent")) i18n_text: "Add agent" width: Fill height: 32 padding: 6 spacing: 5
                    icon_walk: Walk{width: 14 height: 14} draw_icon.svg: ICON_ROBOT}
            }
            agent_picker := View {visible: false width: Fill height: Fit flow: Down spacing: 8
                agent_status := Label {width: Fill height: Fit flow: Flow.Right{wrap: true} draw_text.color: mod.widgets.RINX_MUTED}
                my_agents := RobrixSettingsDropDown {width: Fill height: 40 margin: 0 labels: ["Choose my Agent"]
                    draw_bg.border_radius: 4
                    popup_menu_position: PopupMenuPosition.BelowInput
                    draw_text.text_style: theme.font_regular{font_size: (12 * mod.widgets.RINX_TEXT_SCALE)}
                    popup_menu: RobrixSettingsPopupMenu {width: 256 menu_item: RobrixSettingsPopupMenuItem {height: 36}}}
                invite_selected_agent := RobrixNeutralIconButton {text: "Add selected Agent to this Room" height: 40 icon_walk: Walk{width: 0 height: 0}}
                hide_agent_picker := RobrixNeutralIconButton {text: #(crate::i18n::tr("Hide Agent picker")) height: 32 icon_walk: Walk{width: 0 height: 0}}
                resume_agent_requests := RobrixNeutralIconButton {visible: false text: "Resume original request in Agents" height: 40 icon_walk: Walk{width: 0 height: 0}}
            }
            members_empty := Label {visible: false width: Fill height: Fit text: #(crate::i18n::tr("No members available.")) draw_text.color: mod.widgets.RINX_MUTED}
        }
        list := PortalList {
            width: Fill height: Fill flow: Down
            Header := mod.widgets.MobileSection {
                padding: Inset{left: 16 right: 16 top: 4 bottom: 12} spacing: 6 margin: 0
                Label {width: Fill text: #(crate::i18n::tr("Room details")) draw_text +: {color: mod.widgets.RINX_INK text_style: theme.font_bold{font_size: (12 * mod.widgets.RINX_TEXT_SCALE)}}}
                topic := Label {width: Fill height: Fit flow: Flow.Right{wrap: true}
                    draw_text +: {color: mod.widgets.RINX_INK text_style: theme.font_regular{font_size: (11 * mod.widgets.RINX_TEXT_SCALE)}}}
                View {width: Fill height: 28 flow: Right spacing: 6 align: Align{y: 0.5}
                    room_id := Label {width: Fill height: Fit max_lines: 1 text_overflow: Ellipsis
                        draw_text +: {color: mod.widgets.RINX_MUTED text_style: theme.font_regular{font_size: (9 * mod.widgets.RINX_TEXT_SCALE)}}}
                    copy_room_id := NavigationBarButton {width: 28 height: 28 padding: 6 tooltip_text: #(crate::i18n::tr("Copy Room ID"))
                        Icon {icon_walk: Walk{width: 14 height: 14} draw_icon +: {svg: ICON_COPY color: mod.widgets.RINX_MUTED}}}
                }
            }
            Member := mod.widgets.MobileSection {
                // Clicking a member opens their profile (DM, block, ...).
                row := NavigationBarButton {
                    width: Fill height: 62 flow: Right spacing: 12 padding: Inset{left: 20 right: 20}
                    align: Align{y: 0.5}
                    draw_bg +: {color_hover: mod.widgets.RINX_HOVER color_active: mod.widgets.RINX_HOVER border_radius: 0}
                    avatar := mod.widgets.MobileAvatar {width: 40 height: 40}
                    View {
                        width: Fill height: Fit flow: Down spacing: 3
                        name := Label {
                            width: Fill max_lines: 1 text_overflow: Ellipsis
                            draw_text +: {color: mod.widgets.RINX_INK text_style: theme.font_regular {font_size: (12 * mod.widgets.RINX_TEXT_SCALE)}}
                        }
                        user_id := Label {
                            width: Fill max_lines: 1 text_overflow: Ellipsis
                            draw_text +: {color: mod.widgets.RINX_MUTED text_style: theme.font_regular {font_size: (9 * mod.widgets.RINX_TEXT_SCALE)}}
                        }
                    }
                }
            }
            Action := mod.widgets.MobileSection {
                row := NavigationBarButton {
                    width: Fill height: 56 flow: Right spacing: 10
                    padding: Inset{left: 20 right: 20} align: Align{y: 0.5}
                    draw_bg +: {color_hover: mod.widgets.RINX_HOVER color_active: mod.widgets.RINX_HOVER border_radius: 0}
                    title := Label {
                        width: Fill max_lines: 1 text_overflow: Ellipsis
                        draw_text +: {color: mod.widgets.RINX_INK text_style: theme.font_regular {font_size: (12.5 * mod.widgets.RINX_TEXT_SCALE)}}
                    }
                    value := Label {
                        max_lines: 1
                        draw_text +: {color: mod.widgets.RINX_MUTED text_style: theme.font_regular {font_size: (10 * mod.widgets.RINX_TEXT_SCALE)}}
                    }
                    Icon {
                        icon_walk: Walk{width: 8 height: 13}
                        draw_icon +: {color: mod.widgets.RINX_DISABLED svg: ICON_CHEVRON_RIGHT}
                    }
                }
                mod.widgets.MobileDivider {margin: Inset{left: 20}}
            }
            Filler := View {height: 100 width: Fill}
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum InfoTab { #[default] Overview, Members }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InfoRow { Overview, Member(usize), Notifications, Mode(usize), Back, AgentAccess, AgentServices, Approvals, Filler }
fn info_rows(tab: InfoTab, members: usize, choosing: bool, approvals: bool) -> usize {
    match tab {InfoTab::Members => members, InfoTab::Overview if choosing => 6, InfoTab::Overview => 4 + usize::from(approvals)}
}
fn info_row(tab: InfoTab, members: usize, choosing: bool, approvals: bool, index: usize) -> InfoRow {
    if index >= info_rows(tab, members, choosing, approvals) {return InfoRow::Filler;}
    match tab {
        InfoTab::Members => InfoRow::Member(index),
        InfoTab::Overview if index == 0 => InfoRow::Overview,
        InfoTab::Overview if choosing => if index <= 4 {InfoRow::Mode(index - 1)} else {InfoRow::Back},
        InfoTab::Overview => match index {1 => InfoRow::Notifications, 2 => InfoRow::AgentServices, 3 => InfoRow::AgentAccess, _ => InfoRow::Approvals},
    }
}
#[derive(Clone, Debug)]
pub(crate) struct ChatMembershipChanged {
    pub room_id: OwnedRoomId,
    pub epoch: u64,
}

#[derive(Clone, Debug)]
struct ChatInfoData {
    room: RoomNameId,
    topic: String,
    members: Vec<UserProfile>,
    mode: Option<RoomNotificationMode>,
}

#[derive(Debug)]
struct RoomAgentsLoaded {
    owner: OwnedUserId,
    origin: String,
    epoch: u64,
    room_id: OwnedRoomId,
    request: u64,
    binding: bool,
    result: Result<serde_json::Value, String>,
}
#[derive(Debug)]
struct ChatInfoLoaded {
    owner: OwnedUserId,
    origin: String,
    epoch: u64,
    room_id: OwnedRoomId,
    request: u64,
    result: Result<ChatInfoData, String>,
}

#[derive(Debug)]
struct RoomServicesLoaded {
    owner: OwnedUserId,
    origin: String,
    epoch: u64,
    room_id: OwnedRoomId,
    request: u64,
    result: Result<serde_json::Value, String>,
}

fn room_service_status(service: &serde_json::Value) -> String {
    if service["ownerServicePaused"] == true {return crate::i18n::tr("Paused in this Room. Requests receive a pause notice.").into();}
    if service["bindingState"] == "suspended" {return crate::i18n::tr("Room access unavailable. Check membership and permissions.").into();}
    if service["bindingState"] == "pending" {return crate::i18n::tr("Joining this Room…").into();}
    if service["thisDevice"] != true {return crate::i18n::tr("Not assigned to this device.").into();}
    if service["statusAvailable"] != true {return crate::i18n::tr("Could not verify this Room's service. Refresh to retry.").into();}
    crate::hagency::ui::runtime_summary(&service["runtime"], service["agentId"].as_str().unwrap_or(""), service["bindingId"].as_str())
}

#[derive(Script, ScriptHook, Widget)]
pub struct MobileChatInfo {
    #[source]
    source: ScriptObjectRef,
    #[deref]
    view: View,
    #[rust]
    owner: Option<OwnedUserId>,
    #[rust]
    origin: String,
    #[rust]
    epoch: u64,
    #[rust]
    room: Option<RoomNameId>,
    #[rust]
    request: u64,
    #[rust]
    data: Option<ChatInfoData>,
    #[rust]
    status: String,
    #[rust]
    failed: bool,
    #[rust]
    choosing_notifications: bool,
    #[rust]
    tab: InfoTab,
    #[rust] drawn_tab: InfoTab,
    #[rust] rendered_rows: HashMap<WidgetUid, InfoRow>,
    #[rust] rendered_members: HashMap<WidgetUid, UserProfile>,
    #[rust] agent_request: u64,
    #[rust] agent_busy: bool,
    #[rust] agent_project: String,
    #[rust] agents: Vec<serde_json::Value>,
    #[rust] agent_command: Option<(String, String)>,
    #[rust] agent_picker_open: bool,
    #[rust] services_open: bool,
    #[rust] services_busy: bool,
    #[rust] services_request: u64,
    #[rust] services: Vec<serde_json::Value>,
}

fn is_hagency_service(user: &matrix_sdk::ruma::UserId, owner: &matrix_sdk::ruma::UserId) -> bool {
    user.localpart() == "_hagency_service" && user.server_name() == owner.server_name()
}

fn modes() -> [(Option<RoomNotificationMode>, &'static str); 4] {
    [
        (None, crate::i18n::tr("Use Account Defaults")),
        (
            Some(RoomNotificationMode::AllMessages),
            crate::i18n::tr("All Messages"),
        ),
        (
            Some(RoomNotificationMode::MentionsAndKeywordsOnly),
            crate::i18n::tr("Mentions and Keywords"),
        ),
        (
            Some(RoomNotificationMode::Mute),
            crate::i18n::tr("Mute Notifications"),
        ),
    ]
}

impl MobileChatInfo {
    fn services_response_matches(&self, loaded: &RoomServicesLoaded) -> bool {
        self.owner.as_ref() == Some(&loaded.owner) && self.origin == loaded.origin
            && self.epoch == loaded.epoch && self.services_request == loaded.request
            && self.room.as_ref().is_some_and(|room|room.room_id() == &loaded.room_id)
            && loaded.result.as_ref().map_or(true, |v|v["roomId"] == loaded.room_id.as_str())
    }
    fn load_services(&mut self, cx: &mut Cx) {
        if self.services_busy || !self.pin_is_current() {return;}
        let (Some(owner), Some(room)) = (self.owner.clone(), self.room.clone()) else {return;};
        let Ok(service) = crate::hagency::backend::Service::for_current() else {return;};
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let request = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.services_request = request;
        self.services_busy = true;
        self.services_open = true;
        self.services.clear();
        self.view.drop_down(cx, ids!(room_services)).set_labels(cx, vec![crate::i18n::tr("Choose an Agent").into()]);
        self.view.drop_down(cx, ids!(room_services)).set_selected_item(cx, 0);
        self.view.label(cx, ids!(services_status)).set_text(cx, crate::i18n::tr("Checking Agent services in this Room…"));
        let origin = self.origin.clone(); let epoch = self.epoch; let room_id = room.room_id().clone();
        spawn_async_task(async move {
            let result = service.room_agent_services(room_id.as_str()).await;
            Cx::post_action(RoomServicesLoaded {owner, origin, epoch, room_id, request, result});
        });
        self.view.redraw(cx);
    }
    fn select_tab(&mut self, cx: &mut Cx, tab: InfoTab) {
        self.tab = tab;
        self.choosing_notifications = false;
        self.view.portal_list(cx, ids!(list)).set_first_id_and_scroll(0, 0.0);
        self.view.redraw(cx);
    }
    fn load(&mut self, cx: &mut Cx, room: RoomNameId) {
        self.clear();
        self.fetch_info(cx, room);
    }
    // Refresh metadata without clearing an in-flight Agent invitation or its original key.
    fn fetch_info(&mut self, cx: &mut Cx, room: RoomNameId) {
        self.room = Some(room.clone());
        let epoch = crate::account_session::epoch();
        let Some(client) = get_client() else { return };
        let Some(owner) = client.user_id().map(ToOwned::to_owned) else {
            return;
        };
        if epoch != crate::account_session::epoch() || crate::sliding_sync::account_changing() {
            return;
        }
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        self.request = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.owner = Some(owner.clone());
        let request = self.request;
        self.origin = client.homeserver().to_string();
        self.epoch = epoch;
        let origin = self.origin.clone();
        let epoch = self.epoch;
        let room_id = room.room_id().clone();
        self.status = crate::i18n::tr("Loading chat information…").into();
        self.view
            .portal_list(cx, ids!(list))
            .set_first_id_and_scroll(0, 0.0);
        self.view.redraw(cx);
        spawn_async_task(async move {
            let result = async {
                let room = client
                    .get_room(room.room_id())
                    .ok_or(crate::i18n::tr("This chat is no longer available."))?;
                let members = room
                    .members(RoomMemberships::JOIN)
                    .await
                    .map_err(|e| e.to_string())?;
                let mut members: Vec<_> = members
                    .into_iter()
                    .map(|member| UserProfile {
                        user_id: member.user_id().to_owned(),
                        username: member.display_name().map(ToOwned::to_owned),
                        avatar_state: AvatarState::Known(
                            member.avatar_url().map(ToOwned::to_owned),
                        ),
                    })
                    .collect();
                members.sort_by_key(|p| (p.displayable_name().to_lowercase(), p.user_id.clone()));
                Ok(ChatInfoData {
                    room: RoomNameId::from_room(&room).await,
                    topic: room.topic().unwrap_or_default(),
                    members,
                    mode: room.user_defined_notification_mode().await,
                })
            }
            .await;
            Cx::post_action(ChatInfoLoaded {
                owner,
                origin,
                epoch,
                room_id,
                request,
                result,
            });
        });
    }

    fn pin_is_current(&self) -> bool {
        self.owner == current_user_id()
            && self.epoch == crate::account_session::epoch()
            && !crate::sliding_sync::account_changing()
            && get_client().is_some_and(|client| client.homeserver().as_str() == self.origin)
    }
    fn accepts(&self, loaded: &ChatInfoLoaded) -> bool {
        self.pin_is_current() && self.matches_response(loaded)
    }
    fn matches_response(&self, loaded: &ChatInfoLoaded) -> bool {
        self.owner.as_ref() == Some(&loaded.owner)
            && self.origin == loaded.origin
            && self.epoch == loaded.epoch
            && self.request == loaded.request
            && self
                .room
                .as_ref()
                .is_some_and(|room| room.room_id() == &loaded.room_id)
            && match &loaded.result {
                Ok(data) => data.room.room_id() == &loaded.room_id,
                Err(_) => true,
            }
    }
    fn agent_response_matches(&self, loaded: &RoomAgentsLoaded) -> bool {
        self.owner.as_ref() == Some(&loaded.owner) && self.origin == loaded.origin && self.epoch == loaded.epoch && self.agent_request == loaded.request
            && self.room.as_ref().is_some_and(|room| room.room_id() == &loaded.room_id)
            && loaded.result.as_ref().map_or(true, |value| if loaded.binding {value["creation"]["binding"]["roomId"] == loaded.room_id.as_str()} else {value["roomId"] == loaded.room_id.as_str()})
    }
    fn request_agents(&mut self, cx: &mut Cx, binding: bool) {
        if self.room.as_ref().is_some_and(|room|
            cx.get_global_ref::<RoomsListRef>().is_some_and(|rooms|
                rooms.is_direct_room(room.room_id()) == Some(true))) {
            self.agent_picker_open = false;
            self.view.redraw(cx);
            return;
        }
        if self.agent_busy || !self.pin_is_current() {return;}
        let (Some(owner),Some(room)) = (self.owner.clone(),self.room.clone()) else {return;};
        let service = match crate::hagency::backend::Service::for_current() {Ok(service)=>service,Err(_)=>return};
        let command = if binding {
            let Some(index) = self.view.drop_down(cx,ids!(my_agents)).selected_item().checked_sub(1) else {return;};
            let Some(agent) = self.agents.get(index).and_then(|a| a["id"].as_str()) else {return;};
            if self.agent_project.is_empty() {return;}
            let fingerprint = serde_json::to_string(&(agent,&self.agent_project,room.room_id())).unwrap_or_default();
            if self.agent_command.as_ref().is_none_or(|(old,_)|old != &fingerprint) {self.agent_command=Some((fingerprint,uuid::Uuid::new_v4().to_string()));}
            Some(crate::hagency::model::Command::BindAgent {agent_id:agent.into(),input:serde_json::json!({"projectId":self.agent_project,"roomId":room.room_id(),"idempotencyKey":self.agent_command.as_ref().unwrap().1})})
        } else {None};
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        self.agent_request=NEXT.fetch_add(1,std::sync::atomic::Ordering::Relaxed);
        let request=self.agent_request;
        let origin=self.origin.clone();
        let epoch=self.epoch;
        let room_id=room.room_id().clone();
        self.agent_busy=true;
        self.tab=InfoTab::Members;
        self.agent_picker_open=true;
        self.view.label(cx,ids!(agent_status)).set_text(cx,if binding {"Adding Agent to this Room…"} else {"Checking this Room and your Agents…"});
        if !binding {self.agents.clear(); self.agent_project.clear(); self.view.drop_down(cx,ids!(my_agents)).set_labels(cx,vec!["Choose my Agent".into()]);}
        spawn_async_task(async move {
            let result=if let Some(command)=command {service.execute(command).await} else {service.agents_for_registered_room(room_id.as_str()).await};
            Cx::post_action(RoomAgentsLoaded {owner,origin,epoch,room_id,request,binding,result});
        });
        self.view.redraw(cx);
    }
    fn clear(&mut self) {
        self.owner = None;
        self.origin.clear();
        self.epoch = 0;
        self.room = None;
        self.request = 0;
        self.data = None;
        self.status.clear();
        self.failed = false;
        self.choosing_notifications = false;
        self.tab = InfoTab::Overview;
        self.rendered_rows.clear(); self.rendered_members.clear();
        self.agent_request = 0;
        self.agent_busy = false;
        self.agent_project.clear();
        self.agents.clear();
        self.agent_command = None;
        self.agent_picker_open = false;
        self.services_open = false;
        self.services_busy = false;
        self.services_request = 0;
        self.services.clear();
    }
}

impl Widget for MobileChatInfo {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        if self.owner.is_some() && !self.pin_is_current() {
            self.clear();
        }
        if matches!(event, Event::Signal) {
            crate::avatar_cache::process_avatar_updates(cx);
            self.view.redraw(cx);
        }
        self.view.handle_event(cx, event, scope);
        if let Event::Actions(actions) = event {
            for action in actions {
                if let Some(loaded) = action.downcast_ref::<RoomServicesLoaded>()
                    && self.pin_is_current() && self.services_response_matches(loaded) {
                    self.services_busy = false;
                    match &loaded.result {
                        Ok(value) => {
                            self.services = value["services"].as_array().cloned().unwrap_or_default();
                            let labels = std::iter::once(crate::i18n::tr("Choose an Agent").to_owned()).chain(self.services.iter().map(|s|s["displayName"].as_str().filter(|n|!n.is_empty()).unwrap_or("Agent").to_owned())).collect();
                            self.view.drop_down(cx, ids!(room_services)).set_labels(cx, labels);
                            let selected = usize::from(!self.services.is_empty());
                            self.view.drop_down(cx, ids!(room_services)).set_selected_item(cx, selected);
                            let status = self.services.first().map(room_service_status).unwrap_or_else(||crate::i18n::tr("None of your Agents is bound to this Room. Add one in Members.").into());
                            self.view.label(cx, ids!(services_status)).set_text(cx, &status);
                        }
                        Err(_) => {self.view.label(cx, ids!(services_status)).set_text(cx, crate::i18n::tr("Could not load Agent services. Refresh to retry."));}
                    }
                    self.view.redraw(cx);
                }
                if let Some(update) = action.downcast_ref::<ChatMembershipChanged>()
                    && update.epoch == self.epoch && self.pin_is_current()
                    && self.room.as_ref().is_some_and(|room| room.room_id() == &update.room_id)
                    && let Some(room) = self.room.clone()
                { self.fetch_info(cx, room); }
                if matches!(
                    action.downcast_ref::<LogoutAction>(),
                    Some(LogoutAction::ClearAppState { .. })
                ) {
                    self.clear();
                }
                if let Some(loaded)=action.downcast_ref::<RoomAgentsLoaded>() {
                    if self.pin_is_current() && self.agent_response_matches(loaded) {
                        self.agent_busy=false;
                        match &loaded.result {
                            Ok(value) if loaded.binding=> {
                                self.view.label(cx,ids!(agent_status)).set_text(cx,if value["commandState"] == "pending" {crate::i18n::tr("Binding saved. Check its Room service in Overview.")} else {crate::i18n::tr("Agent added. Open Agent services in Overview to start responding.")});
                            }
                            Ok(value)=> {
                                self.agent_project=value["projectId"].as_str().unwrap_or("").into();
                                self.agents=value["agents"].as_array().map(|rows|rows.iter().filter(|a|a["state"] == "active").cloned().collect()).unwrap_or_default();
                                let labels=std::iter::once("Choose my Agent".to_owned()).chain(self.agents.iter().map(|a|a["displayName"].as_str().filter(|name|!name.is_empty()).unwrap_or("Agent").to_owned())).collect();
                                self.view.drop_down(cx,ids!(my_agents)).set_labels(cx,labels);
                                self.view.drop_down(cx,ids!(my_agents)).set_selected_item(cx,0);
                                self.view.label(cx,ids!(agent_status)).set_text(cx,if self.agents.is_empty() {"No active Agents owned by you are available."} else {"Choose an Agent you own to bind to this Room."});
                            }
                            Err(error)=> {
                                self.view.label(cx,ids!(agent_status)).set_text(cx,match error.as_str() {
                                    "room_project_registration_required"=>"This Room must first be connected to a Project. No Room was registered automatically.",
                                    "room_join_required"=>"Join this Room before adding an Agent.",
                                    "agent_command_recovery_required"=>"An original binding request needs confirmation. Resume it in Agents.",
                                    _ if loaded.binding=>"Could not confirm the binding. Retry the same request, or resume its saved request in Agents.",
                                    _=>"Could not verify this Room and your Agents. Retry when connected.",
                                });
                                self.view.button(cx,ids!(resume_agent_requests)).set_visible(cx,loaded.binding);
                            }
                        }
                        self.view.redraw(cx);
                    }
                }
                if let Some(loaded) = action.downcast_ref::<ChatInfoLoaded>() {
                    if self.accepts(loaded) {
                        match &loaded.result {
                            Ok(data) => {
                                self.data = Some(data.clone());
                                self.failed = false;
                                self.status.clear();
                            }
                            Err(error) => {
                                self.status = crate::i18n::format(
                                    "Could not load chat information: {error}",
                                    &[("error", (error).to_string())],
                                );
                                self.failed = true;
                            }
                        }
                        self.view.redraw(cx);
                    }
                }
                if let Some(update) = action.downcast_ref::<RoomNotificationModeUpdated>() {
                    if let Some(data) = self.data.as_mut() {
                        if data.room.room_id() == &update.room_id {
                            data.mode = update.mode;
                            self.view.redraw(cx);
                        }
                    }
                }
            }
            if self.view.button(cx, ids!(overview_tab)).clicked(actions) {self.select_tab(cx, InfoTab::Overview);return;}
            if self.view.button(cx, ids!(members_tab)).clicked(actions) {self.select_tab(cx, InfoTab::Members);return;}
            if self.tab == InfoTab::Overview && self.services_open && self.view.button(cx, ids!(refresh_services)).clicked(actions) {self.load_services(cx);return;}
            if self.view.drop_down(cx, ids!(room_services)).changed(actions).is_some()
                && let Some(index) = self.view.drop_down(cx, ids!(room_services)).selected_item().checked_sub(1)
                && let Some(service) = self.services.get(index) {
                self.view.label(cx, ids!(services_status)).set_text(cx, &room_service_status(service));
            }
            if self.tab == InfoTab::Overview && self.services_open && self.view.button(cx, ids!(configure_service)).clicked(actions) && self.pin_is_current() && !self.services_busy
                && let Some(index) = self.view.drop_down(cx, ids!(room_services)).selected_item().checked_sub(1)
                && let Some(service) = self.services.get(index)
                && let (Some(agent), Some(binding)) = (service["agentId"].as_str(),service["bindingId"].as_str()) {
                cx.action(crate::hagency::ui::HagencyAction::AgentRun {agent_id:agent.into(),binding_id:binding.into()});
                return;
            }
            if self.view.navigation_bar_button(cx, ids!(history_search)).clicked(actions) || self.view.navigation_bar_button(cx, ids!(history_media)).clicked(actions) {
                if self.pin_is_current() && let Some(data) = self.data.as_ref() {
                    cx.action(super::room_history::RoomHistoryAction::Open {room: data.room.clone(), filter: if self.view.navigation_bar_button(cx, ids!(history_search)).clicked(actions) {super::room_history::HistoryFilter::Messages} else {super::room_history::HistoryFilter::Media}});
                }
                return;
            }
            if self.tab == InfoTab::Members && self.view.button(cx, ids!(invite_people)).clicked(actions) {
                if self.pin_is_current() && let Some(data)=self.data.as_ref() {cx.action(InviteModalAction::Open(data.room.clone()));}
                return;
            }
            if self.view.button(cx, ids!(hide_agent_picker)).clicked(actions) {self.agent_picker_open=false;self.view.redraw(cx);return;}
            if self.tab == InfoTab::Members && self.view.button(cx,ids!(invite_my_agent)).clicked(actions) {self.request_agents(cx,false);return;}
            if self.tab == InfoTab::Members && self.view.button(cx,ids!(invite_selected_agent)).clicked(actions) {self.request_agents(cx,true);return;}
            if self.view.button(cx,ids!(resume_agent_requests)).clicked(actions) {cx.action(crate::hagency::ui::HagencyAction::OpenAgents);return;}
            if self.view.button(cx, ids!(retry)).clicked(actions) {
                if let Some(room) = self.room.clone() {
                    self.load(cx, room);
                }
                return;
            }
            let list = self.view.portal_list(cx, ids!(list));
            for (_index, widget) in list.items_with_actions(actions) {
                if widget.navigation_bar_button(cx, ids!(copy_room_id)).clicked(actions) {
                    if self.pin_is_current() && self.tab == self.drawn_tab
                        && self.rendered_rows.get(&widget.widget_uid()) == Some(&InfoRow::Overview)
                        && let Some(data) = self.data.as_ref() {
                        cx.copy_to_clipboard(data.room.room_id().as_str());
                    }
                    continue;
                }
                if list.was_scrolling()
                    || !widget.navigation_bar_button(cx, ids!(row)).clicked(actions)
                {
                    continue;
                }
                if !self.pin_is_current() {continue;}
                let Some(data) = self.data.as_ref() else {continue;};
                if self.tab != self.drawn_tab {continue;}
                let Some(row) = self.rendered_rows.get(&widget.widget_uid()).copied() else {continue;};
                match row {
                    InfoRow::Member(_) => {
                        let Some(profile)=self.rendered_members.get(&widget.widget_uid()) else {continue;};
                        cx.widget_action(self.widget_uid(), ShowUserProfileAction::ShowUserProfile(UserProfileAndRoomId {user_profile:profile.clone(), room_id:data.room.room_id().clone()}));
                    }
                    InfoRow::Notifications => self.choosing_notifications = true,
                    InfoRow::Mode(mode) => {
                        if let Some((mode, _))=modes().get(mode) {submit_async_request(MatrixRequest::SetRoomNotificationMode {room_id:data.room.room_id().clone(),mode:*mode});}
                    }
                    InfoRow::Back => self.choosing_notifications = false,
                    InfoRow::AgentAccess => cx.action(crate::agent_access::AgentAccessAction::Open),
                    InfoRow::AgentServices => {self.services_open = !self.services_open;if self.services_open {self.load_services(cx);}},
                    InfoRow::Approvals => {
                        #[cfg(feature="agent_chat")]
                        cx.action(crate::agent_chat::approval_inbox::ApprovalInboxAction::Open {project:Some(data.room.room_id().clone())});
                    }
                    InfoRow::Overview | InfoRow::Filler => continue,
                }
                self.view.redraw(cx);
                // PortalList returns an item once per grouped child action.
                // A single pointer event can include hover/focus and click;
                // handle its click only once, especially for Matrix writes.
                break;
            }
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        if self.owner.is_some() && !self.pin_is_current() {
            self.clear();
        }
        let direct = self.room.as_ref().is_some_and(|room|
            cx.get_global_ref::<RoomsListRef>().is_some_and(|rooms|
                rooms.is_direct_room(room.room_id()) == Some(true)));
        let members = self.tab == InfoTab::Members;
        self.view.view(cx, ids!(services_panel)).set_visible(cx, !members && self.services_open);
        let service_selected = self.view.drop_down(cx, ids!(room_services)).selected_item().checked_sub(1).is_some_and(|i| i < self.services.len());
        self.view.button(cx, ids!(configure_service)).set_enabled(cx, service_selected && !self.services_busy && self.pin_is_current());
        self.view.button(cx, ids!(refresh_services)).set_enabled(cx, !self.services_busy);
        self.view.view(cx, ids!(member_controls)).set_visible(cx, members);
        self.view.button(cx,ids!(invite_my_agent)).set_visible(cx,members && !direct);
        self.view.view(cx,ids!(agent_picker)).set_visible(cx,members && self.agent_picker_open && !direct);
        self.view.button(cx, ids!(invite_people)).set_enabled(cx, self.data.is_some() && self.owner.is_some());
        self.view.label(cx, ids!(members_empty)).set_visible(cx, members && self.data.as_ref().is_some_and(|d|d.members.is_empty()));
        let summary_room = self.data.as_ref().map(|data|&data.room).or(self.room.as_ref());
        self.view.label(cx, ids!(summary_name)).set_text(cx, &summary_room.map_or_else(|| crate::i18n::tr("Chat information").to_owned(), |room|room.display().into_owned()));
        let count = self.data.as_ref().map_or_else(|| crate::i18n::tr("Loading members…").to_owned(), |data| crate::i18n::format("{0} members", &[("0",data.members.len().to_string())]));
        self.view.label(cx, ids!(summary_count)).set_text(cx, &count);
        for (id, selected) in [(ids!(overview_tab), !members), (ids!(members_tab), members)] {
            let mut button = self.view.button(cx, id);
            if selected {script_apply_eval!(cx, button, {draw_bg +: {color: mod.widgets.RINX_SELECTED} draw_text +: {color: mod.widgets.RINX_ACCENT}});}
            else {script_apply_eval!(cx, button, {draw_bg +: {color: mod.widgets.RINX_SURFACE} draw_text +: {color: mod.widgets.RINX_INK}});}
        }
        self.view.button(cx,ids!(invite_my_agent)).set_enabled(cx,self.owner.is_some() && !self.agent_busy);
        let selected=self.view.drop_down(cx,ids!(my_agents)).selected_item().checked_sub(1).is_some_and(|i|i<self.agents.len());
        self.view.button(cx,ids!(invite_selected_agent)).set_enabled(cx,selected && !self.agent_project.is_empty() && !self.agent_busy);
        self.view.label(cx, ids!(status)).set_text(cx, &self.status);
        self.view
            .label(cx, ids!(status))
            .set_visible(cx, !self.status.is_empty());
        self.view
            .button(cx, ids!(retry))
            .set_visible(cx, self.failed);
        self.drawn_tab = self.tab;
        self.rendered_rows.clear(); self.rendered_members.clear();
        while let Some(item) = self.view.draw_walk(cx, scope, walk).step() {
            if let Some(mut list) = item.borrow_mut::<PortalList>() {
                let count = self.data.as_ref().map_or(0, |d| info_rows(self.tab, d.members.len(), self.choosing_notifications, cfg!(feature="agent_chat")));
                list.set_item_range(cx, 0, count);
                while let Some(index) = list.next_visible_item(cx) {
                    let Some(data) = self.data.as_mut().filter(|_| index < count) else {
                        list.item(cx, index, id!(Filler)).draw_all(cx, scope);
                        continue;
                    };
                    let row = info_row(self.tab, data.members.len(), self.choosing_notifications, cfg!(feature="agent_chat"), index);
                    let widget = match row {
                        InfoRow::Overview => {
                            let widget = list.item(cx, index, id!(Header));
                            widget.label(cx, ids!(room_id)).set_text(cx, data.room.room_id().as_str());
                            widget.label(cx, ids!(topic)).set_text(cx, if data.topic.is_empty() {crate::i18n::tr("No topic set.")} else {&data.topic});
                            widget
                        }
                        InfoRow::Member(member) => {
                            let Some(profile)=data.members.get_mut(member) else {continue;};
                        let widget = list.item(cx, index, id!(Member));
                        self.rendered_members.insert(widget.widget_uid(), profile.clone());
                        let service = self.owner.as_ref().is_some_and(|owner| is_hagency_service(&profile.user_id, owner));
                        widget.label(cx, ids!(name)).set_text(cx, if service {crate::i18n::tr("Hagency service")} else {profile.displayable_name()});
                        let detail = crate::agent_access::framework_label(&profile.user_id)
                            .map_or_else(
                                || profile.user_id.to_string(),
                                |framework| format!("{framework} · {}", profile.user_id),
                            );
                        widget.label(cx, ids!(user_id)).set_text(cx, &detail);
                        let avatar = widget.avatar(cx, ids!(avatar));
                        let loaded =
                            profile
                                .avatar_state
                                .update_from_cache(cx)
                                .is_some_and(|image| {
                                    avatar
                                        .show_image(cx, None, |cx, img| {
                                            utils::load_avatar_image(&img, cx, image)
                                        })
                                        .is_ok()
                                });
                        if !loaded {
                            avatar.show_user_text(cx, &profile.user_id, profile.displayable_name());
                        }
                        widget
                        }
                        InfoRow::Filler => list.item(cx, index, id!(Filler)),
                        _ => {
                            let widget = list.item(cx, index, id!(Action));
                            let (title, value) = match row {
                                InfoRow::Mode(mode) => modes().get(mode).map(|(mode, title)|(*title, if *mode == data.mode {"✓"} else {""})).unwrap_or(("", "")),
                                InfoRow::Back => (crate::i18n::tr("Back to Chat Info"), ""),
                                InfoRow::Notifications => (crate::i18n::tr("Notifications"), match data.mode {
                                    Some(RoomNotificationMode::Mute) => crate::i18n::tr("Muted"),
                                    Some(RoomNotificationMode::AllMessages) => crate::i18n::tr("All"),
                                    Some(RoomNotificationMode::MentionsAndKeywordsOnly) => crate::i18n::tr("Mentions"),
                                    None => crate::i18n::tr("Default"),
                                }),
                                InfoRow::AgentAccess => (crate::i18n::tr("Agent Access"), ""),
                                InfoRow::AgentServices => (crate::i18n::tr("Agent services"), ""),
                                InfoRow::Approvals => (crate::i18n::tr("Approval rooms"), ""),
                                _ => ("", ""),
                            };
                            widget.label(cx, ids!(title)).set_text(cx, title);
                            widget.label(cx, ids!(value)).set_text(cx, value);
                            widget
                        }
                    };
                    self.rendered_rows.insert(widget.widget_uid(), row);
                    widget.draw_all(cx, scope);
                }
            }
        }
        DrawStep::done()
    }
}

impl MobileChatInfoRef {
    #[cfg(test)]
    pub(crate) fn displayed_room_id(&self) -> Option<OwnedRoomId> {
        self.borrow()
            .and_then(|inner| inner.room.as_ref().map(|room| room.room_id().clone()))
    }
    pub fn ensure_room(&self, cx: &mut Cx, room: RoomNameId) {
        if let Some(mut inner) = self.borrow_mut()
            && (!inner.pin_is_current()
                || inner
                    .room
                    .as_ref()
                    .is_none_or(|current| current.room_id() != room.room_id()))
        {
            inner.load(cx, room);
        }
    }
    pub fn show(&self, cx: &mut Cx, room: RoomNameId) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.load(cx, room);
        }
    }

    pub fn clear(&self) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.clear();
        }
    }
}

#[cfg(test)]
mod information_target_tests {
    use super::*;
    #[test]
    fn explicit_room_pause_and_membership_loss_have_different_service_messages() {
        assert!(room_service_status(&serde_json::json!({"bindingState":"suspended","ownerServicePaused":true})).contains("pause notice"));
        assert!(room_service_status(&serde_json::json!({"bindingState":"suspended","ownerServicePaused":false})).contains("access unavailable"));
    }
    #[test]
    fn room_service_status_never_substitutes_a_running_private_chat() {
        let service = serde_json::json!({"agentId":"agent","bindingId":"project-room","thisDevice":true,"statusAvailable":true,"runtime":{"agentId":"agent","bindingId":"private-chat","phase":"online","bindings":[{"bindingId":"private-chat","phase":"online"}]}});
        assert!(room_service_status(&service).starts_with("This Room is not running"));
        let mut running = service.clone();
        running["runtime"]["bindings"][0]["bindingId"] = "project-room".into();
        assert!(room_service_status(&running).starts_with("Responding in this Room"));
        running["thisDevice"] = false.into();
        assert_eq!(room_service_status(&running), "Not assigned to this device.");
        running["thisDevice"] = true.into();running["statusAvailable"] = false.into();
        assert!(room_service_status(&running).starts_with("Could not verify"));
    }
    #[test]
    fn service_results_cannot_cross_room_or_account_or_request() {
        let mut cx = Cx::new(Box::new(|_,_|{}));
        let mut info = cx.with_vm(|vm| {
            makepad_widgets::script_mod(vm);crate::app::register_widgets(vm);
            let value = script_eval!(vm, {mod.widgets.MobileChatInfo {}});
            MobileChatInfo::script_from_value(vm, value)
        });
        let owner = matrix_sdk::ruma::user_id!("@owner:server").to_owned();
        let room = matrix_sdk::ruma::room_id!("!target:server").to_owned();
        info.owner = Some(owner.clone());info.origin = "https://server/".into();info.epoch = 4;info.room = Some(RoomNameId::empty(room.clone()));info.services_request = 8;
        let mut result = RoomServicesLoaded {owner,origin:info.origin.clone(),epoch:4,room_id:room,request:8,result:Ok(serde_json::json!({"roomId":"!target:server"}))};
        assert!(info.services_response_matches(&result));
        result.result = Ok(serde_json::json!({"roomId":"!other:server"}));assert!(!info.services_response_matches(&result));
        result.result = Err("offline".into());result.request = 7;assert!(!info.services_response_matches(&result));
        result.request = 8;result.epoch = 3;assert!(!info.services_response_matches(&result));
        result.epoch = 4;result.owner = matrix_sdk::ruma::user_id!("@other:server").to_owned();assert!(!info.services_response_matches(&result));
        info.clear();assert!(!info.services_response_matches(&result));
    }
    #[test]
    fn overview_actions_and_member_indices_never_share_notification_or_profile_targets() {
        for members in [0, 1, 5, 200] {
            for choosing in [false, true] {
                assert_eq!(info_row(InfoTab::Members, members, choosing, true, members), InfoRow::Filler);
                for index in 0..members {assert_eq!(info_row(InfoTab::Members, members, choosing, true, index), InfoRow::Member(index));}
                assert_eq!(info_row(InfoTab::Overview, members, choosing, false, 0), InfoRow::Overview);
                if choosing {
                    for mode in 0..4 {assert_eq!(info_row(InfoTab::Overview, members, true, true, mode+1), InfoRow::Mode(mode));}
                    assert_eq!(info_row(InfoTab::Overview, members, true, true, 5), InfoRow::Back);
                    assert_eq!(info_row(InfoTab::Overview, members, true, true, 6), InfoRow::Filler);
                } else {
                    assert_eq!(info_row(InfoTab::Overview, members, false, true, 1), InfoRow::Notifications);
                    assert_eq!(info_row(InfoTab::Overview, members, false, true, 2), InfoRow::AgentServices);
                    assert_eq!(info_row(InfoTab::Overview, members, false, true, 3), InfoRow::AgentAccess);
                    assert_eq!(info_row(InfoTab::Overview, members, false, true, 4), InfoRow::Approvals);
                    assert_eq!(info_row(InfoTab::Overview, members, false, false, 4), InfoRow::Filler);
                }
            }
        }
    }
    #[test]
    fn service_label_identifies_only_exact_service_identity_on_owner_server() {
        let owner=matrix_sdk::ruma::user_id!("@alice:server");
        assert!(is_hagency_service(matrix_sdk::ruma::user_id!("@_hagency_service:server"),owner));
        assert!(!is_hagency_service(matrix_sdk::ruma::user_id!("@_hagency_service:other"),owner));
        assert!(!is_hagency_service(matrix_sdk::ruma::user_id!("@agent:server"),owner));
    }
    fn draw_information(info: &mut MobileChatInfo, cx: &mut Cx) {
        use makepad_widgets::makepad_draw::cx_draw::CxDraw;
        cx.redraw_all();let pass=DrawPass::new(cx);pass.set_size(cx,dvec2(280.0,775.0));
        let mut list=DrawList2d::new(cx);let overlay=Overlay{draw_list:DrawList::new(cx)};
        let event=DrawEvent::default();let mut draw=CxDraw::new(cx,&event);let mut cx2d=Cx2d::new(&mut draw);
        cx2d.begin_pass(&pass,None);list.begin_always(&mut cx2d);overlay.begin(&mut cx2d);
        cx2d.begin_root_turtle(dvec2(280.0,775.0),Layout::flow_overlay());
        info.draw_walk_all(&mut cx2d,&mut Scope::empty(),Walk{width:Size::Fixed(280.0),height:Size::Fixed(775.0),..Walk::default()});
        overlay.end(&mut cx2d);cx2d.end_pass_sized_turtle();list.end(&mut cx2d);cx2d.end_pass(&pass);
    }
    #[test]
    fn registered_room_tabs_keep_summary_fixed_and_original_invitation_intent() {
        let mut cx=Cx::new(Box::new(|_,_|{}));
        let mut info=cx.with_vm(|vm| {
            makepad_widgets::script_mod(vm);vm.bx.captured_errors=Some(Vec::new());crate::app::register_widgets(vm);
            let value=script_eval!(vm,{mod.widgets.MobileChatInfo {}});
            let info=MobileChatInfo::script_from_value(vm,value);assert!(vm.take_errors().is_empty());info
        });
        let room=RoomNameId::new(matrix_sdk::RoomDisplayName::Named("Room A".into()),matrix_sdk::ruma::room_id!("!a:server").to_owned());
        info.room=Some(room.clone());info.data=Some(ChatInfoData{room,topic:"A topic".into(),members:vec![],mode:None});
        info.agent_picker_open=true;info.agent_command=Some(("exact-original-input".into(),"original-key".into()));
        draw_information(&mut info,&mut cx);
        assert_eq!(info.view.label(&mut cx,ids!(summary_name)).text(),"Room A");
        assert!(!info.view.view(&mut cx,ids!(agent_picker)).visible());
        assert!(!info.view.view(&mut cx,ids!(member_controls)).visible());
        assert!(!info.view.navigation_bar_button(&mut cx,ids!(history_search)).is_empty());
        let summary=info.view.view(&mut cx,ids!(summary)).area().rect(&cx);
        info.select_tab(&mut cx,InfoTab::Members);draw_information(&mut info,&mut cx);
        let controls=info.view.view(&mut cx,ids!(member_controls)).area().rect(&cx);
        assert!(controls.pos.y>=summary.pos.y+summary.size.y-1.0,"picker follows fixed summary");
        assert!(info.view.view(&mut cx,ids!(agent_picker)).visible());
        let invite=info.view.button(&mut cx,ids!(invite_people)).area().rect(&cx);
        let agent=info.view.button(&mut cx,ids!(invite_my_agent)).area().rect(&cx);
        assert!((invite.pos.y-agent.pos.y).abs()<1.0,"member actions share a compact row");
        assert!(invite.size.y<=32.1 && agent.size.y<=32.1);
        assert!(invite.pos.x+invite.size.x<=agent.pos.x && agent.pos.x+agent.size.x<=280.0);
        assert!(info.view.portal_list(&mut cx,ids!(list)).area().rect(&cx).size.y>200.0,"invitation controls must leave space for members");
        info.choosing_notifications=true;info.select_tab(&mut cx,InfoTab::Overview);draw_information(&mut info,&mut cx);
        assert!(!info.choosing_notifications);
        assert!(!info.view.view(&mut cx,ids!(agent_picker)).visible());
        assert_eq!(info.agent_command,Some(("exact-original-input".into(),"original-key".into())),"tab navigation doesn't duplicate or discard invitation key");
        assert_eq!(info.view.view(&mut cx,ids!(summary)).area().rect(&cx),summary);
        cx.with_vm(|vm|assert!(vm.take_errors().is_empty()));
        info.clear();assert_eq!(info.tab,InfoTab::Overview);assert!(info.agent_command.is_none());
    }
    #[test]
    fn room_agent_picker_results_cannot_cross_room_account_or_original_call() {
        let mut cx=Cx::new(Box::new(|_,_|{}));
        let mut info=cx.with_vm(|vm| {
            makepad_widgets::script_mod(vm);vm.bx.captured_errors=Some(Vec::new());crate::app::register_widgets(vm);
            let value=script_eval!(vm,{mod.widgets.MobileChatInfo {}});
            let info=MobileChatInfo::script_from_value(vm,value);assert!(vm.take_errors().is_empty());info
        });
        let owner=matrix_sdk::ruma::user_id!("@owner:server").to_owned();
        let room=matrix_sdk::ruma::room_id!("!target:server").to_owned();
        info.owner=Some(owner.clone());info.origin="https://server/".into();info.epoch=4;info.room=Some(RoomNameId::empty(room.clone()));info.agent_request=8;
        let mut result=RoomAgentsLoaded {owner,origin:"https://server/".into(),epoch:4,room_id:room,request:8,binding:false,result:Ok(serde_json::json!({"roomId":"!target:server","projectId":"project"}))};
        assert!(info.agent_response_matches(&result));
        result.result=Ok(serde_json::json!({"roomId":"!other:server"}));assert!(!info.agent_response_matches(&result));
        result.binding=true;result.result=Ok(serde_json::json!({"creation":{"binding":{"roomId":"!target:server"}}}));assert!(info.agent_response_matches(&result));
        result.result=Ok(serde_json::json!({"creation":{"binding":{"roomId":"!other:server"}}}));assert!(!info.agent_response_matches(&result));
        result.result=Err("offline".into());assert!(info.agent_response_matches(&result));
        result.epoch=3;assert!(!info.agent_response_matches(&result));result.epoch=4;
        result.origin="https://other/".into();assert!(!info.agent_response_matches(&result));result.origin="https://server/".into();
        result.request=7;assert!(!info.agent_response_matches(&result));result.request=8;
        result.owner=matrix_sdk::ruma::user_id!("@other:server").to_owned();assert!(!info.agent_response_matches(&result));
        info.clear();assert!(!info.agent_response_matches(&result));
    }
    #[test]
    fn late_members_cannot_cross_room_server_or_account_epoch() {
        let mut cx = Cx::new(Box::new(|_, _| {}));
        let mut info = cx.with_vm(|vm| {
            makepad_widgets::script_mod(vm);
            crate::app::register_widgets(vm);
            let value = script_eval!(vm, {mod.widgets.MobileChatInfo {}});
            MobileChatInfo::script_from_value(vm, value)
        });
        let owner = matrix_sdk::ruma::user_id!("@alice:server").to_owned();
        let room_a = matrix_sdk::ruma::room_id!("!a:server").to_owned();
        let room_b = matrix_sdk::ruma::room_id!("!b:server").to_owned();
        info.owner = Some(owner.clone());
        info.origin = "https://a.test/".into();
        info.epoch = 11;
        info.request = 10;
        info.room = Some(RoomNameId::empty(room_a.clone()));
        let mut result = ChatInfoLoaded {
            owner,
            origin: "https://a.test/".into(),
            epoch: 11,
            room_id: room_a.clone(),
            request: 10,
            result: Ok(ChatInfoData {
                room: RoomNameId::empty(room_a),
                topic: String::new(),
                members: vec![],
                mode: None,
            }),
        };
        assert!(info.matches_response(&result));
        info.room = Some(RoomNameId::empty(room_b.clone()));
        assert!(
            !info.matches_response(&result),
            "same request cannot fill another Room"
        );
        result.room_id = room_b.clone();
        assert!(
            !info.matches_response(&result),
            "payload must agree with requested Room"
        );
        result.result = Err("offline".into());
        assert!(info.matches_response(&result));
        info.epoch = 12;
        assert!(!info.matches_response(&result));
        info.epoch = 11;
        info.origin = "https://b.test/".into();
        assert!(!info.matches_response(&result));
        info.origin = "https://a.test/".into();
        info.clear();
        assert!(!info.matches_response(&result));
    }
}
