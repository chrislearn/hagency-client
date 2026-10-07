'use client';
import { useEffect, useState } from 'react';
import { useSearchParams } from 'next/navigation';
import Link from 'next/link';
import { usePrefs } from '@/components/Prefs';
import OwnerProjectControl from '@/components/OwnerProjectControl';
import { projectRequest, projectName, projectError } from '@/lib/owner-projects';

export default function ProjectsControl() {
  const {locale}=usePrefs(); const text=(zh,en)=>locale.startsWith('zh')?zh:en;
  const selected=useSearchParams().get('project');
  const [projects,setProjects]=useState(null),[rooms,setRooms]=useState(null),[error,setError]=useState(null),[reload,setReload]=useState(0),[roomForm,setRoomForm]=useState(false);
  useEffect(()=>{const controller=new AbortController();setProjects(null);setError(null);projectRequest('owner-projects','GET',undefined,controller.signal).then(value=>setProjects(value.projects)).catch(e=>{if(e.name!=='AbortError')setError(e.message);});return()=>controller.abort();},[reload]);
  useEffect(()=>{setRooms(null);setRoomForm(false);if(!selected)return;const controller=new AbortController();projectRequest(`owner-projects/${encodeURIComponent(selected)}/rooms`,'GET',undefined,controller.signal).then(value=>setRooms(value.rooms)).catch(e=>{if(e.name!=='AbortError')setError(e.message);});return()=>controller.abort();},[selected,reload]);
  const project=projects?.find(p=>p.id===selected);
  return <div className="owner-workspace" data-projects-page>
    <header className="owner-page-heading"><div>{selected&&<Link href="/projects/" className="owner-back">← {text('所有 Projects','All projects')}</Link>}<h1>{selected?(project?projectName(project,text):text('Project','Project')):'Projects'}</h1><p className="dim">{selected?text('管理这个项目的讨论组。每个讨论组单独管理成员和 Agent。','Manage this project’s discussion rooms. Each room has its own members and agents.'):text('组织讨论组与 Agents。每个 Project 绑定一个 Matrix Space。','Organize rooms and agents. Each project is connected to one Matrix Space.')}</p></div>{!selected&&<Link className="btn owner-primary" href="/projects/new/">+ {text('创建 Project','Create project')}</Link>}</header>
    {error&&<div className="owner-notice" role="alert">{projectError(error,text)} <button className="btn" onClick={()=>setReload(n=>n+1)}>{text('重试','Retry')}</button></div>}
    {!projects&&!error&&<p role="status">{text('正在加载 Projects…','Loading projects…')}</p>}
    {projects&&!selected&&(projects.length?<div className="owner-project-grid">{projects.map(p=><Link className="owner-project-card" key={p.id} href={`/projects/?project=${encodeURIComponent(p.id)}`}><div className="owner-project-icon" aria-hidden="true">▦</div><h2>{projectName(p,text)}</h2>{p.topic&&<p>{p.topic}</p>}<p className="owner-space-id">{p.spaceId}</p><span className="owner-card-action">{text('查看讨论组','View rooms')} →</span></Link>)}</div>:<section className="owner-empty"><h2>{text('还没有 Project','No projects yet')}</h2><p>{text('创建一个项目来组织讨论组，也可以绑定已加入的 Matrix Space。','Create a project to organize discussions, or connect a Matrix Space you have joined.')}</p><Link className="btn owner-primary" href="/projects/new/">{text('创建第一个 Project','Create your first project')}</Link></section>)}
    {selected&&projects&&!project&&!error&&<section className="owner-empty"><h2>{text('找不到此 Project','Project not available')}</h2><p>{text('它可能不再对当前账号可见。','It may no longer be visible to this account.')}</p></section>}
    {project&&<><section className="owner-project-overview"><span>{text('关联的 Matrix Space','Connected Matrix Space')}</span><code>{project.spaceId}</code>{project.topic&&<p>{project.topic}</p>}</section>
      <div className="owner-section-heading"><h2>{text('讨论组','Discussion rooms')}</h2><button className="btn owner-primary" onClick={()=>setRoomForm(v=>!v)} aria-expanded={roomForm}>{roomForm?text('取消','Cancel'):text('添加讨论组','Add a room')}</button></div>
      {roomForm&&<OwnerProjectControl key={project.id} project={project} onUpdated={()=>setReload(n=>n+1)} />}
      {!rooms&&!error&&<p role="status">{text('正在加载讨论组…','Loading rooms…')}</p>}
      {rooms&&!rooms.length&&<section className="owner-empty"><h3>{text('还没有可见的讨论组','No visible rooms yet')}</h3><p>{text('新建讨论组，或登记此 Space 下已有的讨论组。Space 的成员不会自动成为讨论组成员。','Create a room or register an existing room under this Space. Space membership does not automatically join its rooms.')}</p></section>}
      {rooms?.length>0&&<div className="owner-room-list">{rooms.map(room=><article className="owner-room-row" key={room.roomId}><div><h3>{room.name||text('未命名讨论组','Untitled room')}</h3><p className="dim">{room.roomId}</p>{room.topic&&<p>{room.topic}</p>}</div><Link className="btn" href="/agents-owned/">{text('管理我的 Agents','Manage my agents')}</Link></article>)}</div>}
    </>}
  </div>;
}
