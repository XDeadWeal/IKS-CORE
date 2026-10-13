fn hash(a: int, b: int) -> float {
    var n = a * 374761393 + b * 668265263;
    n = n ^ (n / 131);
    n = n * 1274126177;
    n = n ^ (n / 33801);
    var m = n % 100000;
    if (m < 0) { m = -m; }
    return float(m) / 100000.0;
}

fn seg_quad(x: float, y: float, w: float, h: float) {
    gl_vertex(x, y, 0.0);
    gl_vertex(x + w, y, 0.0);
    gl_vertex(x + w, y + h, 0.0);
    gl_vertex(x, y + h, 0.0);
}

fn hud_num(val: int, x: float, y: float, dw: float, dh: float, th: float, bits: int[], pw: int[]) {
    var ds: int[] = [];
    var v = val;
    if (v < 0) { v = 0; }
    if (v == 0) { ds = push(ds, 0); }
    while (v > 0) {
        ds = push(ds, v % 10);
        v = v / 10;
    }
    var cx = x;
    var k = len(ds) - 1;
    while (k >= 0) {
        let m = bits[ds[k]];
        if ((m / pw[0]) % 2 == 1) { seg_quad(cx, y + dh - th, dw, th); }
        if ((m / pw[6]) % 2 == 1) { seg_quad(cx, y + (dh - th) * 0.5, dw, th); }
        if ((m / pw[3]) % 2 == 1) { seg_quad(cx, y, dw, th); }
        if ((m / pw[5]) % 2 == 1) { seg_quad(cx, y + (dh - th) * 0.5, th, (dh - th) * 0.5); }
        if ((m / pw[1]) % 2 == 1) { seg_quad(cx + dw - th, y + (dh - th) * 0.5, th, (dh - th) * 0.5); }
        if ((m / pw[4]) % 2 == 1) { seg_quad(cx, y, th, (dh - th) * 0.5); }
        if ((m / pw[2]) % 2 == 1) { seg_quad(cx + dw - th, y, th, (dh - th) * 0.5); }
        cx = cx + dw + th;
        k = k - 1;
    }
}

fn hud_bar(x: float, y: float, bw: float, bh: float, frac: float, r: int, g: int, b: int) {
    gl_color(36, 40, 58, 255);
    seg_quad(x, y, bw, bh);
    var f = frac;
    if (f > 1.0) { f = 1.0; }
    if (f < 0.0) { f = 0.0; }
    if (f > 0.0) {
        gl_color(r, g, b, 255);
        seg_quad(x, y, bw * f, bh);
    }
}

fn main() -> int {
    let min_grid = 8;
    let max_grid = 256;

    gl_init(800, 600, "IKS fur cube stress test");
    gl_depth(true);

    var cr = [220, 60, 70, 230, 200, 70];
    var cg = [60, 200, 110, 200, 80, 210];
    var cb = [60, 90, 230, 60, 210, 210];
    var ccx = [1.0, -1.0, 0.0, 0.0, 0.0, 0.0];
    var ccy = [0.0, 0.0, 1.0, -1.0, 0.0, 0.0];
    var ccz = [0.0, 0.0, 0.0, 0.0, 1.0, -1.0];
    var tux = [0.0, 0.0, 1.0, 1.0, 1.0, 1.0];
    var tuy = [1.0, 1.0, 0.0, 0.0, 0.0, 0.0];
    var tuz = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    var tvx = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    var tvy = [0.0, 0.0, 0.0, 0.0, 1.0, 1.0];
    var tvz = [1.0, 1.0, 1.0, 1.0, 0.0, 0.0];
    var fnx = [1.0, -1.0, 0.0, 0.0, 0.0, 0.0];
    var fny = [0.0, 0.0, 1.0, -1.0, 0.0, 0.0];
    var fnz = [0.0, 0.0, 0.0, 0.0, 1.0, -1.0];
    let bits = [63, 6, 91, 79, 102, 109, 125, 7, 127, 111];
    let pw = [1, 2, 4, 8, 16, 32, 64];

    print("fur cube stress test: ramping load until gpu/cpu saturates");
    print("F11 = fullscreen, ESC or close window = quit");

    var grid = min_grid;
    var angle = 0.0;
    var t_start = gfx_time();
    var t_last = t_start;
    var t_win = t_start;
    var t_title = t_start;
    var win_frames = 0;
    var peak_mt = 0.0;
    var steps = 0;
    var fps_s = 60.0;
    var fs = 0;
    var f11_prev = 0;
    var cpu_now = 0.0;
    var gpu_now = 0.0;
    var cpu_t = t_start;

    while (gl_swap() && !gfx_key_down(27)) {
        let f11 = gfx_key_down(122);
        if (f11 && f11_prev == 0) {
            fs = 1 - fs;
            gl_fullscreen(fs == 1);
        }
        if (f11) { f11_prev = 1; } else { f11_prev = 0; }

        let ww = gl_width();
        let wh = gl_height();
        let ww_f = float(ww);
        let wh_f = float(wh);
        gl_view(0, 0, ww, wh);

        let tnow = gfx_time();
        let dt = tnow - t_last;
        t_last = tnow;
        if (dt > 0.0001 && dt < 1.0) {
            fps_s = fps_s * 0.85 + (1.0 / dt) * 0.15;
        }
        gpu_now = gpu_now * 0.8 + gpu_load() * 0.2;
        if (tnow - cpu_t >= 0.3) {
            cpu_now = cpu_now * 0.5 + cpu_load() * 0.5;
            cpu_t = tnow;
        }

        let step = 2.0 / float(grid);
        let sw = tnow * 1.7;
        let sw2 = tnow * 2.6;
        gl_clear(10, 12, 24);
        gl_proj(60.0, ww_f / wh_f, 0.1, 100.0);
        gl_push();
        gl_translate(0.0, 0.0, -4.2);
        gl_rotate(angle, 0.4, 1.0, 0.3);

        gl_begin(6);
        var f = 0;
        while (f < 6) {
            gl_color(cr[f], cg[f], cb[f], 255);
            let cx = ccx[f];
            let cy = ccy[f];
            let cz = ccz[f];
            let ux = tux[f];
            let uy = tuy[f];
            let uz = tuz[f];
            let vx = tvx[f];
            let vy = tvy[f];
            let vz = tvz[f];
            var j = 0;
            while (j < grid) {
                var v0 = -1.0 + float(j) * step;
                var v1 = v0 + step;
                var i = 0;
                while (i < grid) {
                    var u0 = -1.0 + float(i) * step;
                    var u1 = u0 + step;
                    gl_vertex(cx + u0 * ux + v0 * vx, cy + u0 * uy + v0 * vy, cz + u0 * uz + v0 * vz);
                    gl_vertex(cx + u1 * ux + v0 * vx, cy + u1 * uy + v0 * vy, cz + u1 * uz + v0 * vz);
                    gl_vertex(cx + u1 * ux + v1 * vx, cy + u1 * uy + v1 * vy, cz + u1 * uz + v1 * vz);
                    gl_vertex(cx + u0 * ux + v1 * vx, cy + u0 * uy + v1 * vy, cz + u0 * uz + v1 * vz);
                    i = i + 1;
                }
                j = j + 1;
            }
            f = f + 1;
        }
        gl_end();

        gl_begin(3);
        f = 0;
        while (f < 6) {
            let cx = ccx[f];
            let cy = ccy[f];
            let cz = ccz[f];
            let ux = tux[f];
            let uy = tuy[f];
            let uz = tuz[f];
            let vx = tvx[f];
            let vy = tvy[f];
            let vz = tvz[f];
            let nx = fnx[f];
            let ny = fny[f];
            let nz = fnz[f];
            var j = 0;
            while (j < grid) {
                var v0 = -1.0 + float(j) * step;
                var v1 = v0 + step;
                var i = 0;
                while (i < grid) {
                    var u0 = -1.0 + float(i) * step;
                    var u1 = u0 + step;

                    let hh = hash(i + f * 97, j);
                    let ph = hh * 6.2831853;
                    let len_f = (0.06 + 0.55 * hh) * (0.72 + 0.28 * abs(sin(sw + ph * 1.3)));
                    let bend = 0.12 * sin(sw2 + ph);
                    var cu = (u0 + u1) * 0.5;
                    var cv = (v0 + v1) * 0.5;
                    var ax = cx + cu * ux + cv * vx + nx * len_f + bend;
                    var ay = cy + cu * uy + cv * vy + ny * len_f + bend * 0.45;
                    var az = cz + cu * uz + cv * vz + nz * len_f + bend * 0.8;
                    var shade = 0.3 + 0.7 * hash(j + f * 57, i + 3);
                    gl_color(int(cr[f] * shade), int(cg[f] * shade), int(cb[f] * shade), 255);

                    var c0x = cx + u0 * ux + v0 * vx;
                    var c0y = cy + u0 * uy + v0 * vy;
                    var c0z = cz + u0 * uz + v0 * vz;
                    var c1x = cx + u1 * ux + v0 * vx;
                    var c1y = cy + u1 * uy + v0 * vy;
                    var c1z = cz + u1 * uz + v0 * vz;
                    var c2x = cx + u1 * ux + v1 * vx;
                    var c2y = cy + u1 * uy + v1 * vy;
                    var c2z = cz + u1 * uz + v1 * vz;
                    var c3x = cx + u0 * ux + v1 * vx;
                    var c3y = cy + u0 * uy + v1 * vy;
                    var c3z = cz + u0 * uz + v1 * vz;

                    gl_vertex(c0x, c0y, c0z);
                    gl_vertex(c1x, c1y, c1z);
                    gl_vertex(ax, ay, az);
                    gl_vertex(c1x, c1y, c1z);
                    gl_vertex(c2x, c2y, c2z);
                    gl_vertex(ax, ay, az);
                    gl_vertex(c2x, c2y, c2z);
                    gl_vertex(c3x, c3y, c3z);
                    gl_vertex(ax, ay, az);
                    gl_vertex(c3x, c3y, c3z);
                    gl_vertex(c0x, c0y, c0z);
                    gl_vertex(ax, ay, az);

                    i = i + 1;
                }
                j = j + 1;
            }
            f = f + 1;
        }
        gl_end();
        gl_pop();

        gl_depth(false);
        gl_proj_ortho(0.0, ww_f, 0.0, wh_f);
        gl_begin(6);
        gl_color(16, 18, 30, 255);
        seg_quad(10.0, wh_f - 64.0, 170.0, 56.0);
        gl_color(80, 255, 120, 255);
        hud_num(int(fps_s), 16.0, wh_f - 56.0, 18.0, 40.0, 6.0, bits, pw);
        hud_bar(16.0, wh_f - 90.0, 160.0, 12.0, cpu_now / 100.0, 90, 200, 255);
        gl_color(90, 200, 255, 255);
        hud_num(int(cpu_now), 184.0, wh_f - 92.0, 10.0, 20.0, 3.0, bits, pw);
        hud_bar(16.0, wh_f - 112.0, 160.0, 12.0, gpu_now / 100.0, 255, 120, 220);
        gl_color(255, 120, 220, 255);
        hud_num(int(gpu_now), 184.0, wh_f - 114.0, 10.0, 20.0, 3.0, bits, pw);
        gl_end();
        gl_depth(true);

        angle = angle + 0.5;
        win_frames = win_frames + 1;

        let now = tnow;
        if (now - t_title >= 0.25) {
            let tris_f = 6 * grid * grid * 6;
            let mt = int(float(tris_f) * fps_s / 1000000.0);
            gl_title("fur | fps " + str(int(fps_s))
                   + " | cpu " + str(int(cpu_now)) + "%"
                   + " | gpu " + str(int(gpu_now)) + "%"
                   + " | grid " + str(grid)
                   + " | " + str(mt) + " Mtri/s");
            t_title = now;
        }

        let win_secs = now - t_win;
        if (win_secs >= 1.0) {
            let fps = float(win_frames) / win_secs;
            let tris = 6 * grid * grid * 6;
            let mts = float(tris) * fps / 1000000.0;
            if (mts > peak_mt) {
                peak_mt = mts;
            }
            print("[t", now - t_start, "s] grid", grid,
                  "tris/frame", tris,
                  "fps", fps,
                  "ms", 1000.0 / fps,
                  "Mtri/s", mts,
                  "cpu", cpu_now, "% gpu", gpu_now, "%");

            if (fps >= 20.0 && grid < max_grid) {
                grid = grid + 4;
                steps = steps + 1;
            } else {
                if (fps < 10.0 && grid > min_grid) {
                    grid = grid - 4;
                }
            }
            t_win = now;
            win_frames = 0;
        }
    }
    print("stopped after", gfx_time() - t_start, "seconds,", steps, "load steps, peak", peak_mt, "Mtri/s");
    return 0;
}
