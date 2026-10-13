fn face(a: float[], b: float[], c: float[], d: float[], r: int, g: int, bl: int) {
    gl_color(r, g, bl, 255);
    gl_begin(3);
    gl_vertex(a[0], a[1], a[2]);
    gl_vertex(b[0], b[1], b[2]);
    gl_vertex(c[0], c[1], c[2]);
    gl_vertex(a[0], a[1], a[2]);
    gl_vertex(c[0], c[1], c[2]);
    gl_vertex(d[0], d[1], d[2]);
    gl_end();
}

fn main() -> int {
    gl_init(640, 480, "IKS OpenGL cube");
    gl_view(0, 0, 640, 480);
    gl_proj(60.0, 640.0 / 480.0, 0.1, 100.0);
    gl_depth(true);

    let s = 1.0;
    var v0 = [-s, -s, -s];
    var v1 = [s, -s, -s];
    var v2 = [s, s, -s];
    var v3 = [-s, s, -s];
    var v4 = [-s, -s, s];
    var v5 = [s, -s, s];
    var v6 = [s, s, s];
    var v7 = [-s, s, s];

    var angle = 0.0;
    while (gl_swap()) {
        gl_clear(12, 14, 28);
        gl_push();
        gl_translate(0.0, 0.0, -4.5);
        gl_rotate(angle, 0.5, 1.0, 0.0);
        face(v0, v1, v2, v3, 220, 60, 60);
        face(v4, v5, v6, v7, 60, 200, 90);
        face(v0, v1, v5, v4, 70, 110, 230);
        face(v2, v3, v7, v6, 230, 200, 60);
        face(v1, v2, v6, v5, 200, 80, 210);
        face(v0, v3, v7, v4, 70, 210, 210);
        gl_pop();
        angle = angle + 1.2;
    }
    return 0;
}
