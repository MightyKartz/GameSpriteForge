// Trusted, offline player. Metadata is parsed as data, never evaluated as code.
for (const root of document.querySelectorAll('[data-forge-animation]')) {
  const data = JSON.parse(root.getAttribute('data-forge-animation'));
  const image = root.querySelector('img');
  const select = root.querySelector('[data-animation]');
  const play = root.querySelector('[data-play]');
  const slider = root.querySelector('input');
  const status = root.querySelector('output');
  const background = root.querySelector('[data-background]');
  let active = 0, frame = 0, playing = false, ready = false, origin = 0, raf = 0;
  const images = data.urls.map(src => { const img = new Image(); img.src = src; return img; });
  data.animations.forEach((a, i) => { const option = document.createElement('option'); option.value = i; option.textContent = a.name; select.append(option); });
  function animation() { return data.animations[active]; }
  function elapsed() { return animation().durationsMs.slice(0, frame).reduce((a,b) => a+b, 0); }
  function show() {
    const a = animation(); image.src = data.urls[a.frames[frame]];
    slider.max = a.frames.length - 1; slider.value = frame;
    status.textContent = `${a.name} · ${frame + 1}/${a.frames.length} · ${a.durationsMs[frame].toFixed(1)} ms`;
    play.textContent = playing ? 'Pause' : 'Play';
  }
  function pause() { playing = false; cancelAnimationFrame(raf); show(); }
  function tick(now) {
    if (!playing) return;
    const a = animation(), total = a.durationsMs.reduce((a,b) => a+b, 0);
    let time = Math.max(0, now - origin);
    if (time >= total && !a.loopAnimation) { frame = a.frames.length - 1; pause(); return; }
    time %= total; frame = 0;
    while (frame < a.frames.length - 1 && time >= a.durationsMs[frame]) { time -= a.durationsMs[frame++]; }
    show(); raf = requestAnimationFrame(tick);
  }
  function start() { if (!ready || playing) return; if (frame === animation().frames.length - 1) frame = 0; playing = true; origin = performance.now() - elapsed(); show(); raf = requestAnimationFrame(tick); }
  play.onclick = () => playing ? pause() : start();
  select.onchange = () => { pause(); active = Number(select.value); frame = 0; show(); };
  slider.oninput = () => { pause(); frame = Number(slider.value); show(); };
  root.querySelector('[data-prev]').onclick = () => { pause(); frame = Math.max(0, frame-1); show(); };
  root.querySelector('[data-next]').onclick = () => { pause(); frame = Math.min(animation().frames.length-1, frame+1); show(); };
  background.onchange = () => { image.style.background = ({dark:'#202428',light:'#f0f0f0',checkerboard:'repeating-conic-gradient(#b4b4b4 0% 25%,#dcdcdc 0% 50%) 50% / 32px 32px'})[background.value]; };
  background.onchange(); show();
  Promise.all(images.map(img => img.decode())).then(() => { ready = true; play.disabled = false; start(); }).catch(() => { status.textContent = 'PNG loading failed; reopen the complete preview directory.'; });
}
