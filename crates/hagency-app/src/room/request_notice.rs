//! Render transport status in the viewer's language, independently of the sender.
use serde_json::Value;

pub(crate) fn presentation(content: &Value) -> Option<String> {presentation_for(content,crate::i18n::language())}
fn presentation_for(content: &Value, language:crate::i18n::Language) -> Option<String> {
    if content["msgtype"] != "m.notice" {return None;}
    let notice=&content["org.hagency.request_notice"];
    let kind=if notice["version"] == 1 {notice["kind"].as_str()?} else {legacy_kind(content["body"].as_str()?)?};
    let key=match kind {
        "waiting" => match notice["deviceState"].as_str() {
            Some("offline") => "⏳ Request queued. No recent heartbeat from the execution device. Processing has not started. Check the device and Agent service; this request may expire while waiting.",
            Some("not_running") => "⏳ Request queued. The execution device is connected, but no active Agent runtime was observed. Check Service and its logs. This request may expire while waiting.",
            Some("unassigned") => "⏳ Request queued. No execution device is assigned to this Agent. Assign a device and configure its service. This request may expire while waiting.",
            Some("ready") => "⏳ Request queued. The Agent runtime is connected, but has not started this request. This request may expire while waiting.",
            _ => "⏳ Request queued. Processing has not started. Check the execution device and Service logs. This request may expire while waiting.",
        },
        "processing" => "⏳ The Agent is still processing this request. No need to send it again.",
        "delivery" => "⏳ Processing finished, but the reply has not been delivered. Delivery is being retried.",
        "failed" => "⚠️ This request failed without a reply. Check the Agent's Service logs for the cause.",
        "rejected" => "⏸️ This request was not authorized for processing.",
        "expired" => "⚠️ This request expired before processing started. You can send a new request.",
        "interrupted" => "⚠️ Processing was interrupted and its result is unconfirmed. This request will not be run again automatically.",
        _ => return None,
    };
    let mut text=crate::i18n::text_for(language,key).to_owned();
    if let Some(name)=notice["deviceName"].as_str().filter(|s|!s.trim().is_empty()) {
        let name=name.chars().filter(|c|!c.is_control()).take(128).collect::<String>();
        text.push_str(&format!("\n{}",crate::i18n::text_for(language,"Execution device: {device}").replace("{device}",&name)));
    }
    Some(text)
}

fn legacy_kind(body:&str)->Option<&'static str> {
    Some(match body {
        "⏳ 请求已收到，Agent 当前尚未开始处理（可能离线或正在处理其他请求）。请求仍在等待，无需重复发送；恢复后会继续处理。" => "waiting",
        "⏳ Agent 仍在处理本次请求，耗时较长，请稍候。无需重复发送。" => "processing",
        "⏳ 本次处理已结束，但回复暂未送达，系统正在重试发送。无需重复发送请求。" => "delivery",
        "⚠️ 本次请求处理失败，未生成可发送的回复。后续请求仍可正常处理。" => "failed",
        "⏸️ 本次请求未获处理授权。其他符合授权条件的请求仍可正常处理。" => "rejected",
        "⚠️ 本次请求等待超时，尚未开始处理，已结束等待。你可以重新发送请求。" => "expired",
        "⚠️ 本次请求处理已中断，暂时无法确认执行结果。为避免重复执行，系统不会自动重跑这条请求；你可以继续发送新请求。" => "interrupted",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn structured_notices_follow_viewer_language_without_changing_the_event() {
        for state in ["offline","not_running","unassigned","ready","unobserved"] {
            let content=json!({"msgtype":"m.notice","body":"fallback","org.hagency.request_notice":{"version":1,"kind":"waiting","deviceState":state,"deviceName":"Laptop"}});
            let before=content.clone();
            let english=presentation_for(&content,crate::i18n::Language::English).unwrap();
            let chinese=presentation_for(&content,crate::i18n::Language::Chinese).unwrap();
            assert!(english.contains("Request queued"));assert!(chinese.contains("请求已入队"));
            assert!(chinese.contains("执行设备：Laptop"));assert_eq!(content,before);
        }
    }
    #[test]
    fn only_known_status_notices_replace_text_and_hide_quotes() {
        assert!(presentation(&json!({"msgtype":"m.text","body":"hello"})).is_none());
        assert!(presentation(&json!({"msgtype":"m.notice","body":"ordinary bot reply"})).is_none());
        let old=json!({"msgtype":"m.notice","body":"⏳ 请求已收到，Agent 当前尚未开始处理（可能离线或正在处理其他请求）。请求仍在等待，无需重复发送；恢复后会继续处理。"});
        let text=presentation(&old).unwrap();
        assert!(text.contains("may expire"));assert!(!text.contains("other requests"));
        let mut new=json!({"msgtype":"m.notice","body":"fallback","org.hagency.request_notice":{"version":1,"kind":"waiting","deviceState":"offline","deviceName":"Laptop"}});
        assert!(presentation(&new).unwrap().contains("No recent heartbeat"));
        assert!(presentation(&new).unwrap().contains("Laptop"));
        new["org.hagency.request_notice"]["kind"]=json!("unknown");
        assert!(presentation(&new).is_none());
    }
}
