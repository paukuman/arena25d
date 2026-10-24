import init, { Game } from './pkg/arena25d.js';

const $ = (id) => document.getElementById(id);

async function main() {
  try {
    await init();
  } catch (e) {
    return fail('Gagal memuat WASM: ' + e);
  }
  const canvas = $('c');
  let game;
  try {
    game = new Game(canvas);
  } catch (e) {
    return fail('WebGL2 gagal: ' + e);
  }

  function resize() {
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    const w = Math.floor(window.innerWidth * dpr);
    const h = Math.floor(window.innerHeight * dpr);
    canvas.width = w;
    canvas.height = h;
    game.resize(w, h);
  }
  window.addEventListener('resize', resize);
  resize();

  // --- joystick sentuh (dinamis: muncul di tempat jari menyentuh) ---
  const zone = $('stickzone'), stick = $('stick'), knob = $('knob');
  let stickId = null, ox = 0, oy = 0, sx = 0, sy = 0;
  const R = 50;
  zone.addEventListener('pointerdown', (e) => {
    if (stickId !== null) return;
    stickId = e.pointerId;
    zone.setPointerCapture(stickId);
    ox = e.clientX; oy = e.clientY;
    stick.style.left = ox + 'px'; stick.style.top = oy + 'px'; stick.style.display = 'block';
    knob.style.transform = 'translate(0,0)';
  });
  zone.addEventListener('pointermove', (e) => {
    if (e.pointerId !== stickId) return;
    let dx = e.clientX - ox, dy = e.clientY - oy;
    const l = Math.hypot(dx, dy);
    if (l > R) { dx = dx / l * R; dy = dy / l * R; }
    sx = dx / R; sy = -dy / R;
    knob.style.transform = `translate(${dx}px, ${dy}px)`;
  });
  const endStick = (e) => {
    if (e.pointerId !== stickId) return;
    stickId = null; sx = 0; sy = 0; stick.style.display = 'none';
  };
  zone.addEventListener('pointerup', endStick);
  zone.addEventListener('pointercancel', endStick);

  // --- tombol serang + keyboard (untuk tes di PC) ---
  $('atk').addEventListener('pointerdown', (e) => { e.preventDefault(); game.attack(); });
  const keys = new Set();
  window.addEventListener('keydown', (e) => {
    keys.add(e.code);
    if (e.code === 'Space') game.attack();
  });
  window.addEventListener('keyup', (e) => keys.delete(e.code));
  function keyAxis() {
    const k = (c) => (keys.has(c) ? 1 : 0);
    return [
      k('KeyD') + k('ArrowRight') - k('KeyA') - k('ArrowLeft'),
      k('KeyW') + k('ArrowUp') - k('KeyS') - k('ArrowDown'),
    ];
  }

  $('again').addEventListener('click', () => { game.restart(); $('over').style.display = 'none'; });

  let last = performance.now();
  function loop(now) {
    const [kx, ky] = keyAxis();
    game.set_move(stickId !== null ? sx : kx, stickId !== null ? sy : ky);
    game.frame(now - last);
    last = now;
    $('hp').textContent = 'HP ' + game.hp();
    $('wave').textContent = 'Wave ' + game.wave();
    $('score').textContent = 'Skor ' + game.score();
    if (game.is_dead()) {
      $('final').textContent = 'Skor akhir: ' + game.score();
      $('over').style.display = 'flex';
    }
    requestAnimationFrame(loop);
  }
  requestAnimationFrame(loop);
}

function fail(msg) {
  const el = $('err');
  el.textContent = msg;
  el.style.display = 'flex';
}

main();
