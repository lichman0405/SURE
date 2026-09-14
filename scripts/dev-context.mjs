#!/usr/bin/env node
import fs from 'node:fs'; import path from 'node:path'; import {execFileSync} from 'node:child_process'; import {fileURLToPath} from 'node:url';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const state=JSON.parse(fs.readFileSync(path.join(root,'progress/state.json'),'utf8')); const tasks=JSON.parse(fs.readFileSync(path.join(root,'tasks/tasks.json'),'utf8'));
const byId=new Map(tasks.map(t=>[t.id,t]));
const ready=tasks.filter(t=>['queued','ready'].includes(state.tasks[t.id]?.status)&&(t.depends_on??[]).every(d=>state.tasks[d]?.status==='accepted'||(!byId.get(d)?.required&&['accepted','skipped_nonrequired'].includes(state.tasks[d]?.status))));
let git='unavailable'; try{git=execFileSync('git',['status','--short','--branch'],{cwd:root,encoding:'utf8'}).trim();}catch{}
console.log('SURE autonomous development context'); console.log(`phase=${state.current_phase} status=${state.status}`); console.log(`ready=${ready.slice(0,8).map(t=>t.id).join(', ')||'none'}`); console.log('git='+git.replace(/\n/g,' | ')); console.log('Read MASTER_PROMPT.md and progress/HANDOFF.md before continuing.');
