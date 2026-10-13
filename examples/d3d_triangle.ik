fn main() -> int {
    d3d_init(640, 480, "IKS D3D11 triangle");
    var angle = 0.0;
    while (d3d_present()) {
        d3d_clear(12, 14, 28);
        let c = cos(angle);
        let s = sin(angle);
        var x0 = 0.0;
        var y0 = 0.6;
        var x1 = -0.6;
        var y1 = -0.4;
        var x2 = 0.6;
        var y2 = -0.4;
        var verts = [
            x0 * c - y0 * s, x0 * s + y0 * c, 0.5, 1.0, 0.3, 0.3,
            x1 * c - y1 * s, x1 * s + y1 * c, 0.5, 0.3, 1.0, 0.3,
            x2 * c - y2 * s, x2 * s + y2 * c, 0.5, 0.3, 0.3, 1.0
        ];
        d3d_draw(verts);
        angle = angle + 0.03;
    }
    return 0;
}
