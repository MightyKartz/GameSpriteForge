// Trusted offline player. Same elapsed milliseconds, independent native durations.
// No metadata is evaluated as code. Controls never change source files or reviews.
const players = [];
const sync = document.querySelector('[data-sync]');
const syncStatus = document.querySelector('[data-sync-status]');
let linked = false, raf = 0;
function targets(player) { return linked ? players : [player]; }
function stop(group) { for (const p of group) { p.playing = false; p.show(); } }
function seek(group, ms) { stop(group); for (const p of group) { p.time = ms; p.show(); } }
function start(group) {
  if (!group.every(p => p.ready)) return;
  const now = performance.now();
  for (const p of group) { p.origin = now - p.time; p.playing = true; p.show(); }
  if (!raf) raf = requestAnimationFrame(tick);
}
function tick(now) {
  raf = 0;
  for (const p of players) {
    if (!p.playing) continue;
    p.time = Math.max(0, now - p.origin);
    if (!p.animation().loopAnimation && p.time >= p.total()) {
      p.time = p.total(); p.playing = false;
    }
    p.show();
  }
  if (players.some(p => p.playing)) raf = requestAnimationFrame(tick);
}
for (const root of document.querySelectorAll('[data-forge-animation]')) {
  const data = JSON.parse(root.getAttribute('data-forge-animation'));
  const image = root.querySelector('img'), select = root.querySelector('[data-animation]');
  const play = root.querySelector('[data-play]'), slider = root.querySelector('[data-frame]');
  const status = root.querySelector('output'), background = root.querySelector('[data-background]');
  const scale = root.querySelector('[data-scale]'), guides = root.querySelector('[data-guides]');
  const images = data.urls.map(src => { const img = new Image(); img.src = src; return img; });
  data.animations.forEach((a, i) => { const option = document.createElement('option'); option.value = i; option.textContent = a.name; select.append(option); });
  const p = {data, select, active:0, frame:0, playing:false, ready:false, failed:false, time:0, origin:0,
    animation() { return data.animations[this.active]; },
    total() { return this.animation().durationsMs.reduce((a,b) => a+b, 0); },
    elapsed(index) { return this.animation().durationsMs.slice(0,index).reduce((a,b) => a+b, 0); },
    show() {
      if (this.failed) { status.textContent = 'PNG loading failed; reopen the complete preview directory.'; return; }
      const a = this.animation(), total = this.total();
      let t = a.loopAnimation ? this.time % total : Math.min(this.time,total);
      this.frame = 0;
      while (this.frame < a.frames.length-1 && t >= a.durationsMs[this.frame]) t -= a.durationsMs[this.frame++];
      const index = a.frames[this.frame], zoom = Number(scale.value);
      image.src = data.urls[index];
      image.style.imageRendering = data.textureFilter === 'nearest' ? 'pixelated' : 'auto';
      image.style.width = `${images[index].naturalWidth * zoom}px`;
      image.style.height = `${images[index].naturalHeight * zoom}px`;
      for (const [axis, coord] of [['x',0],['y',1]]) {
        const line = root.querySelector(`.anchor-${axis}`);
        line.hidden = !guides.checked;
        line.style[axis === 'x' ? 'left' : 'top'] = `${data.anchor[coord] * zoom}px`;
      }
      slider.max = a.frames.length-1; slider.value = this.frame;
      status.textContent = `${a.name} · frame ${this.frame} (0-based) / ${a.frames.length} · ${a.durationsMs[this.frame].toFixed(1)} ms · t=${this.time.toFixed(1)} ms`;
      play.textContent = this.playing ? 'Pause' : 'Play';
    }
  };
  players.push(p);
  play.onclick = () => {
    const group = targets(p);
    if (group.some(q => q.playing)) { stop(group); return; }
    // Restart a completed group; otherwise preserve an explicitly selected frame.
    if (group.every(q => !q.animation().loopAnimation && q.time >= q.total())) seek(group,0);
    start(group);
  };
  select.onchange = () => {
    const name = data.animations[Number(select.value)].name, group = targets(p);
    for (const q of group) { q.active = q.data.animations.findIndex(a => a.name === name); q.select.value = q.active; }
    seek(group,0);
  };
  slider.oninput = () => seek(targets(p), p.elapsed(Number(slider.value)));
  root.querySelector('[data-prev]').onclick = () => seek(targets(p), p.elapsed(Math.max(0,p.frame-1)));
  root.querySelector('[data-next]').onclick = () => seek(targets(p), p.elapsed(Math.min(p.animation().frames.length-1,p.frame+1)));
  background.onchange = () => { image.style.background = ({dark:'#202428',light:'#f0f0f0',checkerboard:'repeating-conic-gradient(#b4b4b4 0% 25%,#dcdcdc 0% 50%) 50% / 32px 32px'})[background.value]; };
  scale.onchange = guides.onchange = () => p.show();
  background.onchange(); p.show();
  Promise.all(images.map(img => img.decode())).then(() => {
    p.ready = true; play.disabled = false; p.show();
    if (!linked) start([p]);
  }).catch(() => { p.failed = true; p.show(); });
}
if (sync) {
  sync.disabled = players.length < 2;
  sync.onchange = () => {
    stop(players);
    const common = players.length ? players[0].data.animations.map(a => a.name).filter(name => players.every(p => p.data.animations.some(a => a.name === name))) : [];
    linked = sync.checked && players.length >= 2 && common.length > 0;
    sync.checked = linked;
    syncStatus.textContent = linked ? 'Synchronized elapsed time; each version retains its own frame durations and loop setting.' : (common.length ? 'Independent playback.' : 'No matching animations to synchronize.');
    for (const p of players) {
      for (const option of p.select.options) option.disabled = linked && !common.includes(p.data.animations[Number(option.value)].name);
      if (linked) { p.active = p.data.animations.findIndex(a => a.name === common[0]); p.select.value = p.active; }
    }
    seek(players,0);
  };
}
