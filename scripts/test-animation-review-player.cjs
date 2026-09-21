// Execute the actual embedded player with a controlled clock and minimal DOM.
// This checks time sampling and synchronization, not browser layout or art quality.
const fs = require('node:fs'), vm = require('node:vm'), assert = require('node:assert/strict');
class Element {
  constructor(value='') { this.value=value; this.style={}; this.options=[]; this.checked=false; }
  append(item) { this.options.push(item); }
}
function fixture(durations, loop, names=['idle','attack']) {
  const elements = new Map();
  for (const key of ['img','[data-animation]','[data-play]','[data-frame]','output','[data-background]','[data-scale]','[data-guides]','[data-prev]','[data-next]','.anchor-x','.anchor-y']) elements.set(key,new Element());
  elements.get('[data-play]').disabled=true;
  elements.get('[data-scale]').value='1'; elements.get('[data-background]').value='dark';
  return {elements, querySelector:key=>elements.get(key), getAttribute:()=>JSON.stringify({
    urls:['0.png','1.png','2.png'],anchor:[32,52],textureFilter:'nearest',animations:names.map(name=>({name,frames:[0,1,2],durationsMs:durations,loopAnimation:loop}))})};
}
async function scenario(roots, broken=false) {
  let now=0, nextId=1;
  const callbacks=new Map(), sync=new Element(), status=new Element();
  const ctx={document:{querySelector:s=>s==='[data-sync]'?sync:status,querySelectorAll:()=>roots,createElement:()=>new Element()},
    Image:class {constructor(){this.naturalWidth=64;this.naturalHeight=64;}decode(){return broken ? Promise.reject(new Error('missing PNG')) : Promise.resolve();}},
    performance:{now:()=>now},requestAnimationFrame:fn=>{const id=nextId++;callbacks.set(id,fn);return id;}};
  vm.runInNewContext(fs.readFileSync('packages/core/src/animation_preview_player.js','utf8'),ctx);
  await new Promise(resolve=>setImmediate(resolve));
  const tick=t=>{now=t; const c=[...callbacks.values()];callbacks.clear();c.forEach(fn=>fn(t));};
  return {sync,status,tick,roots};
}
(async()=>{
  const a=fixture([70,150,230],true), b=fixture([100,200,300],false);
  const {sync,tick}=await scenario([a,b]);
  sync.checked=true;sync.onchange();
  const get=(r,k)=>r.elements.get(k);
  get(a,'[data-play]').onclick();tick(80);
  assert.equal(get(a,'[data-frame]').value,1);assert.equal(get(b,'[data-frame]').value,0);
  tick(240);assert.equal(get(a,'[data-frame]').value,2);assert.equal(get(b,'[data-frame]').value,1);
  get(b,'[data-play]').onclick();tick(500);
  assert.match(get(a,'output').textContent,/t=240.0/); // paused together
  get(a,'[data-prev]').onclick();
  assert.match(get(a,'output').textContent,/t=70.0/);assert.match(get(b,'output').textContent,/t=70.0/);
  get(a,'[data-animation]').value='1';get(a,'[data-animation]').onchange();
  assert.match(get(b,'output').textContent,/attack/);assert.match(get(b,'output').textContent,/t=0.0/);
  get(a,'[data-guides]').checked=true;get(a,'[data-guides]').onchange();
  assert.equal(get(a,'img').style.imageRendering,'pixelated');
  assert.equal(get(a,'.anchor-y').style.top,'52px');assert.equal(get(a,'img').style.width,'64px');
  get(a,'[data-scale]').value='2';get(a,'[data-scale]').onchange();
  assert.equal(get(a,'.anchor-y').style.top,'104px');assert.equal(get(a,'img').style.width,'128px');
  get(a,'[data-play]').onclick();tick(1200);
  assert.match(get(a,'output').textContent,/t=700.0/);assert.match(get(b,'output').textContent,/t=600.0/);
  assert.equal(get(b,'[data-frame]').value,2); // no implicit loop for nonlooping version
  sync.checked=false;sync.onchange();
  get(a,'[data-next]').onclick();assert.equal(get(b,'[data-frame]').value,0);
  const none=await scenario([fixture([70,150,230],true,['idle']),fixture([70,150,230],true,['walk'])]);
  none.sync.checked=true;none.sync.onchange();assert.equal(none.sync.checked,false);assert.match(none.status.textContent,/No matching/);
  const broken=await scenario([fixture([70,150,230],true),fixture([70,150,230],true)],true);
  broken.sync.checked=true;broken.sync.onchange();
  get(broken.roots[0],'[data-next]').onclick();
  assert.match(get(broken.roots[0],'output').textContent,/PNG loading failed/);
  assert.equal(get(broken.roots[0],'[data-play]').disabled,true); // only successful decode enables playback
  console.log('PASS shared elapsed time, native durations, hold, seek, action, loop, scale, anchor and independent controls');
})().catch(error=>{console.error(error);process.exitCode=1;});
