fn fib(n: int) -> int {
    if (n < 2) {
        return n;
    }
    return fib(n - 1) + fib(n - 2);
}

fn main() -> int {
    print("Hello from IKS on the web!");
    var i = 0;
    while (i < 10) {
        print("fib of", i, "=", fib(i));
        i = i + 1;
    }
    print("sqrt(2) =", sqrt(2.0), " pi-ish =", floor(3.14159 * 100.0) / 100.0);
    print("7 > 3:", 7 > 3, " true && false:", true && false);
    return 0;
}
