# Arena 2.5D (Rust + WASM + WebGL2)

Game aksi arena 2.5D: kamera miring 45°, joystick sentuh kiri, tombol SERANG kanan.
Di PC: WASD / panah untuk gerak, Spasi untuk serang.

## Build APK lewat GitHub
1. Buat repo baru di GitHub, push semua file ini ke branch `main`.
2. Buka tab **Actions** → workflow **Build APK** (jalan otomatis, atau klik *Run workflow*).
3. Setelah hijau, unduh artifact **arena25d-debug-apk** → ekstrak → `app-debug.apk` → install di HP.

## Tes di PC (opsional)
```
cargo install wasm-pack
wasm-pack build --release --target web --out-dir www/pkg
python3 -m http.server -d www 8000   # buka http://localhost:8000
```
