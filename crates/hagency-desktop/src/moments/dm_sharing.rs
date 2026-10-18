//! Sharing Moments with DM contacts, like WeChat's friend circle.
//!
//! Sharing is on by default and can be turned off in Settings or on the Moments
//! Audience page. An account that shares publishes [`PROFILE_FIELD`] on its Matrix profile
//! (an MSC4133 extended profile field). Its Rinx then keeps its Moments audience
//! in step with its DM contacts:
//! * a DM contact whose profile carries the field is invited to our timeline, and
//! * a Moments invitation from a DM contact is accepted automatically.
//!
//! Requiring the field on both sides limits this to Rinx users who opted in, so
//! contacts on other Matrix clients never receive a Moments invitation they can't use.
//! A contact who has ever been a member of our timeline (including one we removed)
//! is never re-invited.
use std::{
    collections::{BTreeSet, HashMap},
    sync::Mutex,
    time::{Duration, Instant},
};

use anyhow::Result;
use makepad_widgets::{Cx, log, warning};
use matrix_sdk::{
    Client, RoomMemberships,
    ruma::{
        OwnedUserId, UserId,
        api::client::profile::{ProfileFieldName, ProfileFieldValue},
    },
};

use super::backend::Service;

/// The profile field announcing that an account shares Moments with its DM contacts.
pub const PROFILE_FIELD: &str = "rs.robius.rinx.moments";
const PROFILE_VALUE: &str = "dm_contacts";

/// How long after login the background loop first reconciles the audience, and how often after that.
const FIRST_RUN: Duration = Duration::from_secs(60);
const LOOP_INTERVAL: Duration = Duration::from_secs(5 * 60);
/// How long a contact's profile lookup is trusted before asking again.
/// A contact who opts in later isn't left waiting for this: their own Rinx invites us,
/// and accepting that invitation re-checks them right away (see `sync_dm_sharing`).
const PROFILE_TTL: Duration = Duration::from_secs(30 * 60);

/// Cached answers to "does this contact share Moments with DM contacts?".
static SHARES: Mutex<Option<HashMap<OwnedUserId, (bool, Instant)>>> = Mutex::new(None);

/// The current account's sharing setting as last loaded or changed, for the Settings screens.
static SETTING: Mutex<Option<(OwnedUserId, bool)>> = Mutex::new(None);

/// The account whose profile field this process has already made sure is published.
static PUBLISHED: Mutex<Option<OwnedUserId>> = Mutex::new(None);

/// Posted whenever the sharing setting is loaded or changes, so Settings screens can redraw.
#[derive(Clone, Debug)]
pub struct SharingSettingChanged;

fn store_setting(owner: &UserId, share: bool) {
    *SETTING.lock().unwrap() = Some((owner.to_owned(), share));
    Cx::post_action(SharingSettingChanged);
}

/// The current account's sharing setting, or `None` if it hasn't been loaded yet.
pub fn sharing_setting() -> Option<bool> {
    let owner = crate::sliding_sync::current_user_id()?;
    SETTING
        .lock()
        .unwrap()
        .as_ref()
        .filter(|(user, _)| *user == owner)
        .map(|(_, share)| *share)
}

/// Changes the sharing setting from a Settings screen. The new value shows immediately;
/// if saving fails, an error pops up and the saved value is shown again.
pub fn change_sharing_setting(share: bool) {
    let Some(service) = Service::current() else { return };
    store_setting(&service.owner, share);
    crate::sliding_sync::spawn_async_task(async move {
        if let Err(e) = service.set_share_with_dm_contacts(share).await {
            crate::shared::popup_list::enqueue_popup_notification(
                crate::i18n::format("Could not update Moments sharing: {e}", &[("e", e.to_string())]),
                crate::shared::popup_list::PopupKind::Error,
                Some(5.0),
            );
            if let Ok(prefs) = service.preferences().await {
                store_setting(&service.owner, prefs.share_with_dm_contacts);
            }
        }
    });
}

/// The users we share a joined direct-message room with.
pub fn dm_contacts(client: &Client, owner: &UserId) -> BTreeSet<OwnedUserId> {
    client
        .joined_rooms()
        .into_iter()
        .filter(|room| !super::is_moments(room))
        .flat_map(|room| room.direct_targets())
        .filter_map(|target| UserId::parse(target.as_str()).ok())
        .filter(|id| id != owner)
        .collect()
}

/// Whether `user` publishes the Moments-sharing profile field.
/// Lookup failures (including servers without extended profiles) count as "no".
async fn shares_with_dm_contacts(client: &Client, user: &UserId) -> bool {
    if let Some(cache) = SHARES.lock().unwrap().as_ref()
        && let Some((shares, at)) = cache.get(user)
        && at.elapsed() < PROFILE_TTL
    {
        return *shares;
    }
    let shares = matches!(
        client
            .account()
            .fetch_profile_field_of(user.to_owned(), ProfileFieldName::from(PROFILE_FIELD))
            .await,
        Ok(Some(value)) if value.value().as_str() == Some(PROFILE_VALUE)
    );
    SHARES
        .lock()
        .unwrap()
        .get_or_insert_with(HashMap::new)
        .insert(user.to_owned(), (shares, Instant::now()));
    shares
}

impl Service {
    /// Turns sharing with DM contacts on or off: saves the preference
    /// and publishes or removes the profile field.
    pub async fn set_share_with_dm_contacts(&self, share: bool) -> Result<()> {
        self.update_preferences(|prefs| prefs.share_with_dm_contacts = share)
            .await?;
        store_setting(&self.owner, share);
        if share {
            self.publish_profile_field().await?;
            self.sync_dm_sharing().await?;
        } else {
            self.client
                .account()
                .delete_profile_field(ProfileFieldName::from(PROFILE_FIELD))
                .await?;
            *PUBLISHED.lock().unwrap() = None;
        }
        Ok(())
    }

    /// Publishes the sharing profile field (at most once per process, per account).
    async fn publish_profile_field(&self) -> Result<()> {
        if PUBLISHED.lock().unwrap().as_ref() == Some(&self.owner) {
            return Ok(());
        }
        let published = self
            .client
            .account()
            .fetch_profile_field_of(self.owner.clone(), ProfileFieldName::from(PROFILE_FIELD))
            .await
            .is_ok_and(|v| v.is_some_and(|v| v.value().as_str() == Some(PROFILE_VALUE)));
        if !published {
            self.client
                .account()
                .set_profile_field(ProfileFieldValue::new(
                    PROFILE_FIELD,
                    serde_json::json!(PROFILE_VALUE),
                )?)
                .await?;
        }
        *PUBLISHED.lock().unwrap() = Some(self.owner.clone());
        Ok(())
    }

    /// Accepts Moments invitations from DM contacts and invites DM contacts who
    /// share the same way. Does nothing unless this account opted in.
    ///
    /// Returns how many contacts were invited.
    pub async fn sync_dm_sharing(&self) -> Result<usize> {
        let prefs = self.preferences().await?;
        store_setting(&self.owner, prefs.share_with_dm_contacts);
        if !prefs.share_with_dm_contacts {
            return Ok(0);
        }
        // Sharing is on by default, so nobody flips a switch to publish the field.
        self.publish_profile_field().await?;
        let contacts = dm_contacts(&self.client, &self.owner);

        for room in self.client.invited_rooms() {
            if !super::is_moments(&room) {
                continue;
            }
            if let Ok(invite) = room.invite_details().await
                && contacts.contains(&invite.inviter_id)
            {
                self.invitation(room.room_id(), true).await?;
                // Their invitation shows they share now; forget any cached "no" so that
                // they are invited back below, rather than after the cache expires.
                if let Some(cache) = SHARES.lock().unwrap().as_mut() {
                    cache.remove(&invite.inviter_id);
                }
            }
        }

        // Anyone with any membership record (joined, invited, left, removed) is skipped,
        // so removing a viewer or declining sticks.
        let known = |room: Option<matrix_sdk::Room>| async move {
            match room {
                Some(room) => room
                    .members_no_sync(RoomMemberships::all())
                    .await
                    .map(|members| members.into_iter().map(|m| m.user_id().to_owned()).collect()),
                None => Ok(BTreeSet::new()),
            }
        };
        let existing = prefs.timeline.as_ref().and_then(|id| self.client.get_room(id));
        let already: BTreeSet<OwnedUserId> = known(existing).await?;
        let mut sharing = vec![];
        for contact in contacts.iter().filter(|c| !already.contains(*c)) {
            if shares_with_dm_contacts(&self.client, contact).await {
                sharing.push(contact.clone());
            }
        }
        // Only now create a timeline if needed, so accounts with no Rinx contacts get no empty room.
        if sharing.is_empty() {
            return Ok(0);
        }
        let timeline = self.ensure_timeline().await?;
        let members: BTreeSet<OwnedUserId> = known(self.client.get_room(&timeline)).await?;
        let mut invited = 0;
        for contact in sharing.iter().filter(|c| !members.contains(*c)) {
            self.membership(&timeline, contact, true).await?;
            invited += 1;
        }
        Ok(invited)
    }
}

/// Periodically reconciles the Moments audience with DM contacts while logged in.
/// Spawned alongside the other sync tasks, and aborted with them on logout.
pub async fn dm_sharing_loop() {
    // Load the setting early so the Settings screens can show it before the first reconcile.
    tokio::time::sleep(Duration::from_secs(5)).await;
    if let Some(service) = Service::current()
        && let Ok(prefs) = service.preferences().await
    {
        store_setting(&service.owner, prefs.share_with_dm_contacts);
    }
    let mut wait = FIRST_RUN;
    loop {
        tokio::time::sleep(wait).await;
        wait = LOOP_INTERVAL;
        if let Some(service) = Service::current() {
            match service.sync_dm_sharing().await {
                Ok(0) => {}
                Ok(n) => log!("Moments: invited {n} DM contact(s) to the audience."),
                Err(e) => warning!("Moments: couldn't sync sharing with DM contacts: {e}"),
            }
        }
    }
}
