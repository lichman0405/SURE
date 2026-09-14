#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const ROOT=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const required=[
  'MASTER_PROMPT.md','CLAUDE.md','ASSUMPTIONS.md','START_HERE.md','Cargo.toml','rust-toolchain.toml',
  'docs/product/PRODUCT_THESIS.md','docs/product/MVP_SPEC.md','docs/product/DEFINITION_OF_DONE.md',
  'docs/architecture/ARCHITECTURE.md','docs/architecture/PROJECT_INTENT.md','docs/architecture/EXECUTION_SAFETY.md',
  'docs/security/THREAT_MODEL.md','docs/testing/TEST_STRATEGY.md','docs/development/WINDOWS.md',
  'tasks/phases.json','tasks/tasks.json','progress/state.json','progress/HANDOFF.md'
];
let errors=[];
for(const rel of required) if(!fs.existsSync(path.join(ROOT,rel))) errors.push(`Missing required file: ${rel}`);
const tasks=JSON.parse(fs.readFileSync(path.join(ROOT,'tasks/tasks.json'),'utf8'));
const phases=JSON.parse(fs.readFileSync(path.join(ROOT,'tasks/phases.json'),'utf8'));
const state=JSON.parse(fs.readFileSync(path.join(ROOT,'progress/state.json'),'utf8'));
const ids=new Set();
for(const t of tasks){ if(ids.has(t.id)) errors.push(`Duplicate task ${t.id}`); ids.add(t.id); }
for(const t of tasks) for(const d of t.depends_on??[]) if(!ids.has(d)) errors.push(`${t.id} depends on missing ${d}`);
const byId=new Map(tasks.map(t=>[t.id,t])); const visiting=new Set(), visited=new Set();
function visit(id){ if(visited.has(id)) return; if(visiting.has(id)){errors.push(`Task dependency cycle at ${id}`);return;} visiting.add(id); for(const d of byId.get(id)?.depends_on??[]) visit(d); visiting.delete(id); visited.add(id); }
for(const id of ids) visit(id);
for(const id of ids) if(!Object.prototype.hasOwnProperty.call(state.tasks,id)) errors.push(`progress/state.json missing ${id}`);
for(const id of Object.keys(state.tasks)) if(!ids.has(id)) errors.push(`progress/state.json has unknown task ${id}`);
const phaseIds=new Set(phases.map(p=>p.id));
for(const t of tasks) if(!phaseIds.has(t.phase)) errors.push(`${t.id} uses missing phase ${t.phase}`);
if(errors.length){ console.error(errors.join('\n')); process.exit(2); }
console.log(`SURE bootstrap validation OK: ${phases.length} phases, ${tasks.length} tasks.`);
