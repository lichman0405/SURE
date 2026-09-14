#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const ROOT=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const taskPath=path.join(ROOT,'tasks/tasks.json'), statePath=path.join(ROOT,'progress/state.json');
const tasks=JSON.parse(fs.readFileSync(taskPath,'utf8'));
let state=JSON.parse(fs.readFileSync(statePath,'utf8'));
const byId=new Map(tasks.map(t=>[t.id,t]));
const now=()=>new Date().toISOString();
const save=()=>{state.last_updated=now(); const tmp=statePath+'.tmp'; fs.writeFileSync(tmp,JSON.stringify(state,null,2)+'\n'); fs.renameSync(tmp,statePath);};
function ready(t){ return ['queued','ready'].includes(state.tasks[t.id]?.status) && (t.depends_on??[]).every(d=>state.tasks[d]?.status==='accepted' || (!byId.get(d)?.required && ['skipped_nonrequired','accepted'].includes(state.tasks[d]?.status))); }
function noteArg(args){ const i=args.indexOf('--note'); return i>=0 ? args.slice(i+1).join(' ') : ''; }
const [cmd,...args]=process.argv.slice(2);
if(!cmd || cmd==='status'){
 const counts={}; for(const v of Object.values(state.tasks)) counts[v.status]=(counts[v.status]||0)+1;
 console.log(`Project: ${state.project} | status: ${state.status} | phase: ${state.current_phase}`);
 console.log(counts); const r=tasks.filter(ready); if(r.length) console.log('READY:',r.map(t=>t.id).join(', ')); process.exit(0);
}
if(cmd==='ready'){ const r=tasks.filter(ready); for(const t of r) console.log(`${t.id}\t${t.title}`); process.exit(0); }
if(cmd==='validate'){ const unknown=Object.keys(state.tasks).filter(id=>!byId.has(id)); const missing=tasks.filter(t=>!state.tasks[t.id]); if(unknown.length||missing.length){console.error({unknown,missing:missing.map(t=>t.id)});process.exit(2);} console.log(`state OK: ${tasks.length} tasks`); process.exit(0); }
const id=args[0]; if(!id || !byId.has(id)){console.error('Known task id required');process.exit(2);} const rec=state.tasks[id]; const task=byId.get(id);
if(cmd==='start'){
 if(!ready(task)){console.error(`${id} is not READY`);process.exit(2);} rec.status='in_progress'; rec.started_at=rec.started_at||now(); rec.notes=noteArg(args)||rec.notes||''; state.current_phase=task.phase; state.status='in_progress'; save(); console.log(`${id} started`); process.exit(0);
}
if(cmd==='accept'){
 if(!['in_progress','ready','queued'].includes(rec.status)){console.error(`${id} cannot be accepted from ${rec.status}`);process.exit(2);} rec.status='accepted'; rec.finished_at=now(); const n=noteArg(args); if(n) rec.notes=n; save(); console.log(`${id} accepted`); process.exit(0);
}
if(cmd==='block-external'){
 rec.status='blocked_external'; rec.finished_at=now(); rec.notes=noteArg(args)||'External blocker not described'; state.blocked_external=[...new Set([...(state.blocked_external||[]),id])]; save(); console.log(`${id} blocked_external`); process.exit(0);
}
if(cmd==='skip'){
 if(task.required){console.error('Cannot skip a required task');process.exit(2);} rec.status='skipped_nonrequired'; rec.finished_at=now(); rec.notes=noteArg(args)||rec.notes||''; save(); console.log(`${id} skipped_nonrequired`); process.exit(0);
}
console.error(`Unknown command: ${cmd}`); process.exit(2);
