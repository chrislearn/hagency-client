import {test as nodeTest} from 'node:test';
const test = process.env.VITEST ? (await import('vitest')).test : nodeTest;
import assert from 'node:assert/strict';
import {mkdtempSync,writeFileSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {checkSpecBindings} from './check-spec-bindings.mjs';
const withTree=(files,check)=>{const root=mkdtempSync(path.join(tmpdir(),'owned-spec-'));try{for(const [name,body] of Object.entries(files))writeFileSync(path.join(root,name),body);check(root);}finally{rmSync(root,{recursive:true,force:true});}};
const header='tags: [rust]\n---\n';
test('superseded product and retired scenarios remain visible while mixed SDK selectors still fail closed',()=>withTree({'owner.spec.md':header+'Test: owner_actual\n','old.spec.md':'superseded-by: owner.spec.md\n'+header+'Test: obsolete\n','mixed.spec.md':header+'Retired-Test: retired_product\nTest: sdk_valid\nTest: sdk_missing\n'},directory=>{const result=checkSpecBindings([{name:'owner_actual'},{name:'sdk_valid'}],{directory});assert.deepEqual(result.missing.map(x=>x.selector),['sdk_missing']);assert.equal(result.superseded.length,1);assert.equal(result.retired.length,1);assert.equal(result.count,3);}));
for(const [name,files] of Object.entries({missing:{'old.spec.md':'superseded-by: missing.spec.md\n'+header},self:{'old.spec.md':'superseded-by: old.spec.md\n'+header},traversal:{'old.spec.md':'superseded-by: ../owner.spec.md\n'+header},chain:{'old.spec.md':'superseded-by: other.spec.md\n'+header,'other.spec.md':'superseded-by: owner.spec.md\n'+header,'owner.spec.md':header}}))test(`superseded spec ${name} is refused`,()=>withTree(files,directory=>assert.throws(()=>checkSpecBindings([],{directory}),/superseded-by|Superseded-by/)));

test('a superseded product cannot redirect to an empty replacement',()=>withTree({'old.spec.md':'superseded-by: owner.spec.md\n'+header,'owner.spec.md':header},directory=>assert.throws(()=>checkSpecBindings([],{directory}),/Empty superseded-by replacement/)));
