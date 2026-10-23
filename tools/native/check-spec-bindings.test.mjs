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

test('platform-scoped bindings defer only their own selector and remain required on each declared platform', () => withTree({
  'mixed.spec.md': header + 'Scenario: Unix fixture\n  Test-Platforms: [linux, macos]\n  Test: unix_fixture\nScenario: Portable refusal\n  Test: portable_refusal\n',
}, directory => {
  const windows = checkSpecBindings([{ name: 'portable_refusal' }], { directory, platform: 'win32' });
  assert.deepEqual(windows.missing, []);
  assert.equal(windows.count, 1);
  assert.equal(windows.platformDeferred[0].selector, 'unix_fixture');
  assert.deepEqual(checkSpecBindings([], { directory, platform: 'win32' }).missing.map(x => x.selector), ['portable_refusal']);
  for (const platform of ['linux', 'darwin']) {
    const absent = checkSpecBindings([{ name: 'portable_refusal' }], { directory, platform });
    assert.deepEqual(absent.missing.map(x => x.selector), ['unix_fixture']);
    assert.equal(absent.platformDeferred.length, 0);
    assert.deepEqual(checkSpecBindings([{ name: 'unix_fixture' }, { name: 'portable_refusal' }], { directory, platform }).missing, []);
  }
}));

test('a scoped Filter does not leak its platform to the next scenario', () => withTree({
  'mixed.spec.md': header + 'Scenario: Windows\n  Test-Platforms: [windows]\n  Filter: windows_fixture\nScenario: Shared\n  Test: shared_fixture\n',
}, directory => {
  const linux = checkSpecBindings([], { directory, platform: 'linux' });
  assert.deepEqual(linux.missing.map(x => x.selector), ['shared_fixture']);
  assert.deepEqual(linux.platformDeferred.map(x => x.selector), ['windows_fixture']);
  assert.deepEqual(checkSpecBindings([], { directory, platform: 'win32' }).missing.map(x => x.selector), ['windows_fixture', 'shared_fixture']);
}));

for (const declaration of ['[unix]', '[]', '[linux, linux]', '[linux,]', '[Linux]', 'linux', '[linux] trailing']) {
  test(`invalid test platform ${declaration} is refused`, () => withTree({
    'bad.spec.md': header + `Test-Platforms: ${declaration}\nTest: original\n`,
  }, directory => assert.throws(() => checkSpecBindings([{ name: 'original' }], { directory, platform: 'win32' }), /Invalid Test-Platforms/)));
}

for (const following of ['', 'Scenario: Other\nTest: original', 'Retired-Test: original', 'Test-Platforms: [windows]\nTest: original']) {
  test(`orphan or duplicate test platform ${JSON.stringify(following)} is refused`, () => withTree({
    'bad.spec.md': header + `Test-Platforms: [linux]\n${following}\n`,
  }, directory => assert.throws(() => checkSpecBindings([], { directory, platform: 'win32' }), /Invalid Test-Platforms/)));
}
