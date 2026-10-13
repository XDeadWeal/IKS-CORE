struct Vec2 {
    x: float,
    y: float,
}

fn add(a: int, b: int) -> int {
    return a + b;
}

fn length2(v: Vec2) -> float {
    return v.x * v.x + v.y * v.y;
}

fn test_add() -> bool {
    return add(2, 3) == 5 && add(-2, 2) == 0;
}

fn test_length2() -> bool {
    let v = Vec2 { x: 3.0, y: 4.0 };
    return length2(v) == 25.0;
}

fn test_assert_smoke() {
    assert(1 == 1, "one equals one");
}

fn main() -> int {
    print("run 'ik test' to execute the test suite");
    return 0;
}
