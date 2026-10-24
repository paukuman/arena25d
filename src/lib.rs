use js_sys::{Float32Array, Uint16Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    HtmlCanvasElement, WebGl2RenderingContext as GL, WebGlProgram, WebGlShader,
    WebGlUniformLocation, WebGlVertexArrayObject,
};

type V3 = [f32; 3];
type M4 = [f32; 16];

const ARENA: f32 = 11.0;
const SQ: f32 = std::f32::consts::FRAC_1_SQRT_2;

const VS: &str = r#"#version 300 es
layout(location=0) in vec3 a_pos;
layout(location=1) in vec3 a_nrm;
uniform mat4 u_vp;
uniform vec3 u_pos;
uniform vec3 u_scale;
uniform float u_rot;
out vec3 v_nrm;
void main() {
    float c = cos(u_rot);
    float s = sin(u_rot);
    vec3 l = a_pos * u_scale;
    vec3 w = vec3(c * l.x + s * l.z, l.y, -s * l.x + c * l.z) + u_pos;
    v_nrm = vec3(c * a_nrm.x + s * a_nrm.z, a_nrm.y, -s * a_nrm.x + c * a_nrm.z);
    gl_Position = u_vp * vec4(w, 1.0);
}"#;

const FS: &str = r#"#version 300 es
precision mediump float;
in vec3 v_nrm;
uniform vec3 u_color;
out vec4 o;
void main() {
    vec3 n = normalize(v_nrm);
    float d = max(dot(n, normalize(vec3(-0.4, 1.0, 0.5))), 0.0);
    o = vec4(u_color * (0.45 + 0.65 * d), 1.0);
}"#;

// ---------- matematika kecil ----------
fn sub(a: V3, b: V3) -> V3 { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
fn dot(a: V3, b: V3) -> f32 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }
fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn norm(a: V3) -> V3 {
    let l = dot(a, a).sqrt().max(1e-6);
    [a[0] / l, a[1] / l, a[2] / l]
}
fn perspective(fovy: f32, aspect: f32, near: f32, far: f32) -> M4 {
    let f = 1.0 / (fovy / 2.0).tan();
    let mut m = [0.0; 16];
    m[0] = f / aspect;
    m[5] = f;
    m[10] = (far + near) / (near - far);
    m[11] = -1.0;
    m[14] = 2.0 * far * near / (near - far);
    m
}
fn look_at(eye: V3, center: V3, up: V3) -> M4 {
    let f = norm(sub(center, eye));
    let s = norm(cross(f, up));
    let u = cross(s, f);
    [
        s[0], u[0], -f[0], 0.0,
        s[1], u[1], -f[1], 0.0,
        s[2], u[2], -f[2], 0.0,
        -dot(s, eye), -dot(u, eye), dot(f, eye), 1.0,
    ]
}
fn mul(a: &M4, b: &M4) -> M4 {
    let mut o = [0.0; 16];
    for c in 0..4 {
        for r in 0..4 {
            let mut s = 0.0;
            for k in 0..4 { s += a[k * 4 + r] * b[c * 4 + k]; }
            o[c * 4 + r] = s;
        }
    }
    o
}

// ---------- WebGL helper ----------
fn compile(gl: &GL, kind: u32, src: &str) -> Result<WebGlShader, String> {
    let sh = gl.create_shader(kind).ok_or("create_shader gagal")?;
    gl.shader_source(&sh, src);
    gl.compile_shader(&sh);
    if gl.get_shader_parameter(&sh, GL::COMPILE_STATUS).as_bool().unwrap_or(false) {
        Ok(sh)
    } else {
        Err(gl.get_shader_info_log(&sh).unwrap_or_default())
    }
}
fn link(gl: &GL, vs: &WebGlShader, fs: &WebGlShader) -> Result<WebGlProgram, String> {
    let p = gl.create_program().ok_or("create_program gagal")?;
    gl.attach_shader(&p, vs);
    gl.attach_shader(&p, fs);
    gl.link_program(&p);
    if gl.get_program_parameter(&p, GL::LINK_STATUS).as_bool().unwrap_or(false) {
        Ok(p)
    } else {
        Err(gl.get_program_info_log(&p).unwrap_or_default())
    }
}
fn cube_mesh() -> (Vec<f32>, Vec<u16>) {
    let faces: [(V3, V3, V3); 6] = [
        ([1., 0., 0.], [0., 1., 0.], [0., 0., 1.]),
        ([-1., 0., 0.], [0., 1., 0.], [0., 0., 1.]),
        ([0., 1., 0.], [1., 0., 0.], [0., 0., 1.]),
        ([0., -1., 0.], [1., 0., 0.], [0., 0., 1.]),
        ([0., 0., 1.], [1., 0., 0.], [0., 1., 0.]),
        ([0., 0., -1.], [1., 0., 0.], [0., 1., 0.]),
    ];
    let mut v: Vec<f32> = Vec::new();
    let mut idx: Vec<u16> = Vec::new();
    for (fi, (n, u, w)) in faces.iter().enumerate() {
        for (su, sw) in [(-1.0f32, -1.0f32), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            for k in 0..3 {
                v.push(n[k] * 0.5 + u[k] * 0.5 * su + w[k] * 0.5 * sw);
            }
            v.extend_from_slice(n);
        }
        let b = (fi * 4) as u16;
        idx.extend_from_slice(&[b, b + 1, b + 2, b, b + 2, b + 3]);
    }
    (v, idx)
}

struct Gfx {
    gl: GL,
    _vao: WebGlVertexArrayObject,
    n_idx: i32,
    u_vp: Option<WebGlUniformLocation>,
    u_pos: Option<WebGlUniformLocation>,
    u_scale: Option<WebGlUniformLocation>,
    u_rot: Option<WebGlUniformLocation>,
    u_color: Option<WebGlUniformLocation>,
}

impl Gfx {
    fn draw(&self, pos: V3, scale: V3, rot: f32, col: V3) {
        let g = &self.gl;
        g.uniform3f(self.u_pos.as_ref(), pos[0], pos[1], pos[2]);
        g.uniform3f(self.u_scale.as_ref(), scale[0], scale[1], scale[2]);
        g.uniform1f(self.u_rot.as_ref(), rot);
        g.uniform3f(self.u_color.as_ref(), col[0], col[1], col[2]);
        g.draw_elements_with_i32(GL::TRIANGLES, self.n_idx, GL::UNSIGNED_SHORT, 0);
    }
}

struct Enemy {
    x: f32,
    z: f32,
    hp: i32,
    flash: f32,
}

#[wasm_bindgen]
pub struct Game {
    gfx: Gfx,
    w: f32,
    h: f32,
    px: f32,
    pz: f32,
    face: f32,
    hp: i32,
    cd: f32,
    swing: f32,
    hurt_cd: f32,
    cam_x: f32,
    cam_z: f32,
    mx: f32,
    my: f32,
    attack_req: bool,
    enemies: Vec<Enemy>,
    score: u32,
    spawn_t: f32,
    rng: u32,
    dead: bool,
}

#[wasm_bindgen]
impl Game {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: HtmlCanvasElement) -> Result<Game, JsValue> {
        let gl = canvas
            .get_context("webgl2")?
            .ok_or("WebGL2 tidak tersedia")?
            .dyn_into::<GL>()?;
        let vs = compile(&gl, GL::VERTEX_SHADER, VS)?;
        let fs = compile(&gl, GL::FRAGMENT_SHADER, FS)?;
        let prog = link(&gl, &vs, &fs)?;
        gl.use_program(Some(&prog));

        let (verts, idx) = cube_mesh();
        let vao = gl.create_vertex_array().ok_or("vao gagal")?;
        gl.bind_vertex_array(Some(&vao));
        let vb = gl.create_buffer().ok_or("buffer gagal")?;
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&vb));
        unsafe {
            gl.buffer_data_with_array_buffer_view(
                GL::ARRAY_BUFFER,
                &Float32Array::view(&verts),
                GL::STATIC_DRAW,
            );
        }
        let ib = gl.create_buffer().ok_or("buffer gagal")?;
        gl.bind_buffer(GL::ELEMENT_ARRAY_BUFFER, Some(&ib));
        unsafe {
            gl.buffer_data_with_array_buffer_view(
                GL::ELEMENT_ARRAY_BUFFER,
                &Uint16Array::view(&idx),
                GL::STATIC_DRAW,
            );
        }
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_with_i32(0, 3, GL::FLOAT, false, 24, 0);
        gl.enable_vertex_attrib_array(1);
        gl.vertex_attrib_pointer_with_i32(1, 3, GL::FLOAT, false, 24, 12);
        gl.enable(GL::DEPTH_TEST);

        let gfx = Gfx {
            u_vp: gl.get_uniform_location(&prog, "u_vp"),
            u_pos: gl.get_uniform_location(&prog, "u_pos"),
            u_scale: gl.get_uniform_location(&prog, "u_scale"),
            u_rot: gl.get_uniform_location(&prog, "u_rot"),
            u_color: gl.get_uniform_location(&prog, "u_color"),
            n_idx: idx.len() as i32,
            _vao: vao,
            gl,
        };
        Ok(Game {
            gfx,
            w: canvas.width() as f32,
            h: canvas.height() as f32,
            px: 0.0, pz: 0.0, face: 0.0, hp: 10,
            cd: 0.0, swing: 0.0, hurt_cd: 0.0,
            cam_x: 0.0, cam_z: 0.0,
            mx: 0.0, my: 0.0, attack_req: false,
            enemies: Vec::new(), score: 0, spawn_t: 1.0,
            rng: 0x1234_5678, dead: false,
        })
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        self.w = w as f32;
        self.h = h as f32;
        self.gfx.gl.viewport(0, 0, w as i32, h as i32);
    }

    /// x: kanan positif, y: atas (maju) positif, rentang -1..1
    pub fn set_move(&mut self, x: f32, y: f32) { self.mx = x; self.my = y; }
    pub fn attack(&mut self) { self.attack_req = true; }

    pub fn restart(&mut self) {
        self.px = 0.0; self.pz = 0.0; self.hp = 10; self.score = 0;
        self.enemies.clear(); self.dead = false; self.spawn_t = 1.0;
    }

    pub fn hp(&self) -> i32 { self.hp.max(0) }
    pub fn score(&self) -> u32 { self.score }
    pub fn wave(&self) -> u32 { 1 + self.score / 100 }
    pub fn is_dead(&self) -> bool { self.dead }

    pub fn frame(&mut self, dt_ms: f32) {
        let dt = (dt_ms / 1000.0).min(0.05);
        self.update(dt);
        self.render();
    }

    fn rnd(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (x >> 8) as f32 / 16_777_216.0
    }

    fn update(&mut self, dt: f32) {
        self.cd -= dt;
        self.swing -= dt;
        self.hurt_cd -= dt;
        if self.dead {
            self.attack_req = false;
            return;
        }

        // gerak: layar-atas = arah menjauhi kamera (kamera di +x,+z)
        let (mut ix, mut iy) = (self.mx, self.my);
        let len = (ix * ix + iy * iy).sqrt();
        if len > 1.0 { ix /= len; iy /= len; }
        let dx = SQ * ix - SQ * iy;
        let dz = -SQ * ix - SQ * iy;
        if len > 0.1 {
            self.px = (self.px + dx * 6.0 * dt).clamp(-ARENA, ARENA);
            self.pz = (self.pz + dz * 6.0 * dt).clamp(-ARENA, ARENA);
            self.face = dx.atan2(dz);
        }

        // serangan
        if self.attack_req && self.cd <= 0.0 {
            self.cd = 0.45;
            self.swing = 0.2;
            let (fx, fz) = (self.face.sin(), self.face.cos());
            for e in self.enemies.iter_mut() {
                let (ex, ez) = (e.x - self.px, e.z - self.pz);
                let d = (ex * ex + ez * ez).sqrt().max(0.001);
                if d < 2.6 && (ex * fx + ez * fz) / d > -0.3 {
                    e.hp -= 1;
                    e.flash = 0.15;
                    e.x += ex / d * 1.5;
                    e.z += ez / d * 1.5;
                }
            }
            let before = self.enemies.len();
            self.enemies.retain(|e| e.hp > 0);
            self.score += ((before - self.enemies.len()) as u32) * 10;
        }
        self.attack_req = false;

        // musuh
        let wave = self.wave();
        let speed = 2.2 + 0.1 * wave as f32;
        let (px, pz) = (self.px, self.pz);
        let mut hit = false;
        for e in self.enemies.iter_mut() {
            e.flash = (e.flash - dt).max(0.0);
            let (dx, dz) = (px - e.x, pz - e.z);
            let d = (dx * dx + dz * dz).sqrt().max(0.001);
            if d > 0.9 {
                e.x += dx / d * speed * dt;
                e.z += dz / d * speed * dt;
            } else if self.hurt_cd <= 0.0 {
                hit = true;
            }
        }
        if hit {
            self.hp -= 1;
            self.hurt_cd = 0.8;
            if self.hp <= 0 { self.dead = true; }
        }

        // spawn
        self.spawn_t -= dt;
        let cap = 5 + wave as usize * 2;
        if self.spawn_t <= 0.0 && self.enemies.len() < cap {
            let side = (self.rnd() * 4.0) as u32;
            let t = self.rnd() * 2.0 * ARENA - ARENA;
            let (x, z) = match side {
                0 => (t, -ARENA),
                1 => (t, ARENA),
                2 => (-ARENA, t),
                _ => (ARENA, t),
            };
            self.enemies.push(Enemy { x, z, hp: 2, flash: 0.0 });
            self.spawn_t = (2.0 - 0.12 * wave as f32).max(0.5);
        }

        // kamera mengikuti pemain
        let k = (5.0 * dt).min(1.0);
        self.cam_x += (self.px - self.cam_x) * k;
        self.cam_z += (self.pz - self.cam_z) * k;
    }

    fn render(&self) {
        let g = &self.gfx.gl;
        g.clear_color(0.04, 0.08, 0.13, 1.0);
        g.clear(GL::COLOR_BUFFER_BIT | GL::DEPTH_BUFFER_BIT);

        // kamera 45 derajat (pitch 45, yaw 45)
        let (yaw, pitch, dist) = (45f32.to_radians(), 45f32.to_radians(), 26.0f32);
        let t = [self.cam_x, 0.0, self.cam_z];
        let eye = [
            t[0] + dist * pitch.cos() * yaw.sin(),
            dist * pitch.sin(),
            t[2] + dist * pitch.cos() * yaw.cos(),
        ];
        let proj = perspective(0.55, self.w / self.h.max(1.0), 1.0, 100.0);
        let vp = mul(&proj, &look_at(eye, t, [0.0, 1.0, 0.0]));
        g.uniform_matrix4fv_with_f32_array(self.gfx.u_vp.as_ref(), false, &vp);

        let d = &self.gfx;
        // lantai
        for i in 0..6 {
            for j in 0..6 {
                let x = -10.0 + 4.0 * i as f32;
                let z = -10.0 + 4.0 * j as f32;
                let c = if (i + j) % 2 == 0 { [0.16, 0.22, 0.30] } else { [0.19, 0.26, 0.35] };
                d.draw([x, -0.25, z], [4.0, 0.5, 4.0], 0.0, c);
            }
        }
        // dinding
        let wc = [0.35, 0.30, 0.22];
        d.draw([0.0, 0.4, -12.5], [26.0, 0.8, 1.0], 0.0, wc);
        d.draw([0.0, 0.4, 12.5], [26.0, 0.8, 1.0], 0.0, wc);
        d.draw([-12.5, 0.4, 0.0], [1.0, 0.8, 26.0], 0.0, wc);
        d.draw([12.5, 0.4, 0.0], [1.0, 0.8, 26.0], 0.0, wc);

        // bayangan + musuh
        for e in &self.enemies {
            d.draw([e.x, 0.02, e.z], [1.2, 0.02, 1.2], 0.0, [0.05, 0.07, 0.1]);
            let col = if e.flash > 0.0 { [1.0, 1.0, 1.0] } else { [0.8, 0.2, 0.25] };
            let rot = (self.px - e.x).atan2(self.pz - e.z);
            d.draw([e.x, 0.7, e.z], [1.0, 1.4, 1.0], rot, col);
            d.draw([e.x, 1.6, e.z], [0.6, 0.5, 0.6], rot, [0.9, 0.35, 0.3]);
        }

        // pemain
        d.draw([self.px, 0.02, self.pz], [1.1, 0.02, 1.1], 0.0, [0.05, 0.07, 0.1]);
        let hurt = self.hurt_cd > 0.5;
        let body = if hurt { [1.0, 0.3, 0.3] } else { [0.25, 0.55, 0.95] };
        d.draw([self.px, 0.8, self.pz], [0.9, 1.2, 0.6], self.face, body);
        d.draw([self.px, 1.7, self.pz], [0.6, 0.6, 0.6], self.face, [0.96, 0.78, 0.6]);
        if self.swing > 0.0 {
            let p = 1.0 - self.swing / 0.2;
            let r = 2.6 * p;
            let (fx, fz) = (self.face.sin(), self.face.cos());
            d.draw([self.px, 0.06, self.pz], [r * 2.0, 0.05, r * 2.0], 0.0, [0.96, 0.75, 0.34]);
            d.draw(
                [self.px + fx * 1.2, 1.0, self.pz + fz * 1.2],
                [0.3, 0.3, 1.6],
                self.face,
                [0.9, 0.9, 0.95],
            );
        }
    }
}
