#!/usr/bin/env python3
"""Authored native Robrix adapter for the AppCard single-atlas workflow.

Coordinates are manually inspected reference regions (atlas pixels), never
inferred business logic or a claimed one-pixel geometry pass. Html is a native
role missing from the upstream static image compiler, so this adapter binds
actual Robrix widgets, L0 state and Matrix services instead of compiling WASM.
"""
import hashlib, json
from pathlib import Path
ROOT=Path(__file__).resolve().parent

def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def write(path,value):path.write_text(json.dumps(value,ensure_ascii=False,indent=2)+'\n')
# IDs, roles, native widgets, absolute reference bounds, authored behavior.
MAP={
'received-card': [('open_mini_app','button','MiniAppCard',[80,234,218,109],'Open admitted package details; no automatic execution'),('card_title','text','Label',[151,252,135,20],'Localized built-in app name'),('card_origin','text','Label',[151,277,135,18],'Markdown / native preview')],
'app-details':[('article_heading','text','Label',[411,62,105,20],'App details'),('details','layout','View',[334,95,281,510],'Built-in app identity and version'),('article_continue','button','Button',[347,526,255,54],'Display current Matrix account and consent; no grant yet')],
'account-consent':[('consent_account','text','Label',[735,138,143,43],'Trusted host current account, never sender identity'),('consent','layout','View',[655,222,250,275],'Local drafts and per-send publish permission disclosure'),('article_cancel','button','Button',[655,526,120,48],'Close without grant or draft read'),('article_allow','button','Button',[787,526,120,48],'Create in-memory account/session/instance grant; load only own draft')],
'markdown-editor':[('article_title','input','TextInput',[959,120,255,44],'L0 title / title_changed'),('article_bold','button','Button',[974,193,27,23],'Insert Markdown at current selection'),('article_italic','button','Button',[1021,193,24,23],'Insert Markdown at current selection'),('article_h2','button','Button',[1069,193,25,23],'Insert Markdown heading'),('article_markdown','input','TextInput',[959,227,254,280],'L0 markdown / markdown_changed; multiline'),('article_status','text','Label',[959,548,105,22],'Actual saved/error status, never simulated'),('article_preview','button','Button',[1100,529,114,45],'Sanitize and render immutable native Html preview'),('article_share','button','Button',[1161,63,58,20],'Choose chat then host-confirm sharing package reference only')],
'native-preview':[('article_html','rich_text','Html',[43,735,254,320],'Ruma Markdown -> sanitized HTML; no images/links/scripts or browser'),('article_publish','button','Button',[43,1125,252,51],'Choose a joined chat; no send until final host confirmation')],
'room-picker':[('article_chat_search','input','TextInput',[348,715,253,39],'Case-insensitive local joined chat name filter'),('article_rooms','collection','PortalList',[348,775,253,200],'Host-owned joined non-Moments room list; exact room ID selection')],
'publish-confirmation':[('publish_account','text','Label',[717,726,177,41],'Captured grant owner'),('publish_room','text','Label',[717,800,171,38],'Selected room display name; exact Matrix ID bound separately'),('confirm_html','rich_text','Html',[655,867,252,237],'Exact immutable content that will be sent'),('article_change','button','Button',[655,1126,119,48],'Return to room selection without sending'),('article_confirm','button','Button',[787,1126,120,48],'Recheck grant/membership/power; send captured content with stable transaction ID')],
'published':[('success_title','text','Label',[1050,888,96,32],'Success only after SDK accepts send'),('success_room','text','Label',[1000,947,210,21],'Actual destination'),('article_edit','button','Button',[960,1057,252,52],'Return to own draft'),('article_share_again','button','Button',[960,1120,252,53],'Start host-controlled share flow')]
}
manifest=json.loads((ROOT/'image-to-appcard-flow.json').read_text())
for scene in manifest['scenes']:
    directory=ROOT/scene['directory']; provenance=json.loads((directory/'atlas-provenance.json').read_text())
    transform=provenance['transform']['atlas_to_logical'];a,_,c,_,e,f=transform
    elements=[]
    for ident,role,widget,bounds,behavior in MAP[scene['id']]:
        x,y,w,h=bounds
        elements.append(dict(id=ident,role=role,native_widget=widget,reference_atlas_bounds=bounds,reference_logical_bounds=[round(a*x+c,2),round(e*y+f,2),round(a*w,2),round(e*h,2)],measurement='manual reference inspection; approximate region, not glyph metrics',behavior=behavior,source='src/mini_app.rs' if scene['id']=='received-card' else 'src/article_app/ui.rs'))
    contract=dict(schema_version=1,id=scene['design_id'],adapter='robrix-native-article-v1',artboard=manifest['artboard'],font_family='PingFang SC via Robrix theme on macOS/iOS',palette={'page':'ffffff','header':'ededed','ink':'191919','muted':'777777','accent':'07c160'},reference_sha256=sha(directory/'reference.png'),elements=elements)
    write(directory/'contract.json',contract)
    write(directory/'semantic-map.json',dict(schema_version=1,adapter=contract['adapter'],reference_sha256=contract['reference_sha256'],contract_sha256=sha(directory/'contract.json'),elements=elements,raster_widgets=False,visual_acceptance=False))
    write(directory/'service-actions.json',dict(schema_version=1,trust_boundary='Robrix host; L0 has text state only',actions=[{'id':x['id'],'behavior':x['behavior']} for x in elements if x['role'] in ('button','input','collection')]))
    (directory/'AUTHORING.md').write_text('Authored reference mapping uses the Robrix native adapter, not the upstream static Studio compiler. See ../../README.md. Approximate reference regions are retained separately from real widget dumps; no automatic visual score.\n')
print(json.dumps({'authored_scenes':len(MAP),'adapter':'robrix-native-article-v1','visual_acceptance':False}))
