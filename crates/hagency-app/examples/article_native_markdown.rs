//! Offline native Markdown fixture. No browser, account, network or clipboard IO.
use makepad_widgets::*;
app_main!(App);

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*
    startup() do #(App::script_component(vm)) {
        ui: Root {
            main_window := Window {
                window.inner_size: vec2(600, 760)
                body +: {
                    flow: Down padding: 20 spacing: 12
                    ScrollYView {width: Fill height: Fill
                        native_body := Html {
                            width: Fill height: Fit selectable: true font_size: 14
                            draw_selection +: {color: #x3399ff55}
                            text_style_normal: theme.font_regular
                            text_style_bold: theme.font_bold
                            text_style_italic: theme.font_italic
                            rmath := ArticleMath {} rdiagram := ArticleDiagram {}
                            rimage := ArticleImage {} remoji := ArticleEmoji {}
                            rcode := ArticleCode {} rcell := ArticleCell {}
                        }
                    }
                    inspect := Button {text: "Inspect native copy" grab_key_focus: false}
                    result := Label {width: Fill height: 40 text: "Ready"}
                }
            }
        }
    }
}

const SOURCE: &str = "Alpha 中文 **bold** and *italic*.\n\n:smiley: and $E=mc^2$.\n\n```rust\nfn main() {\n    println!(\"你好\");\n}\n```\n\n| Left | Right |\n| :--- | ---: |\n| cell | 234 |\n\nFinal paragraph.";

#[derive(Script, ScriptHook)]
pub struct App {
    #[live]
    ui: WidgetRef,
}
impl MatchEvent for App {
    fn handle_startup(&mut self, cx: &mut Cx) {
        let images = article_makepad::content::Images::default();
        let mut renderer = article_makepad::content::NativeRenderer {
            images: &images,
            size: 14.0,
            ink: 0x191919,
        };
        let source = format!("{SOURCE}\n\n```seq\nAndrew->China: Says Hello\nNote right of China: China thinks\\nabout it\nChina-->Andrew: How are you?\nAndrew->>China: I am good thanks!\n```\n\n```mermaid\nflowchart LR\nA[中文] --> B[Complete]\n```");
        let blocks = article_core::markdown_render::render(&source, &mut renderer);
        let html = blocks
            .iter()
            .map(|b| article_makepad::content::native_html(&b.html))
            .collect::<String>();
        self.ui.html(cx, ids!(native_body)).set_text(cx, &html);
    }
    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        if self.ui.button(cx, ids!(inspect)).clicked(actions) {
            let response = std::rc::Rc::new(std::cell::RefCell::new(None));
            self.ui.handle_event(
                cx,
                &Event::TextCopy(TextClipboardEvent {
                    response: response.clone(),
                }),
                &mut Scope::empty(),
            );
            let text = self
                .ui
                .html(cx, ids!(native_body))
                .borrow()
                .map(|h| h.get_full_text())
                .unwrap_or_default();
            self.ui.label(cx, ids!(result)).set_text(
                cx,
                &serde_json::json!({"copy":*response.borrow(),"text":text}).to_string(),
            );
        }
    }
}
impl AppMain for App {
    fn script_mod(vm: &mut ScriptVm) -> ScriptValue {
        makepad_widgets::theme_mod(vm);
        script_eval!(vm,{mod.theme=mod.themes.light});
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        article_makepad::apple_fonts::install(vm);
        makepad_widgets::widgets_mod(vm);
        article_makepad::script_mod(vm);
        self::script_mod(vm)
    }
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        self.match_event(cx, event);
        self.ui.handle_event(cx, event, &mut Scope::empty());
    }
}
