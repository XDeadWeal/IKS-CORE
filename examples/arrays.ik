fn sum(xs: int[]) -> int {
    var total = 0;
    var i = 0;
    while (i < len(xs)) {
        total = total + xs[i];
        i = i + 1;
    }
    return total;
}

fn main() -> int {
    var xs = [3, 1, 4, 1, 5, 9, 2, 6];
    print("array:", xs);
    print("sum:", sum(xs));

    var words = ["iks", "makes", "arrays", "easy"];
    var i = 0;
    while (i < len(words)) {
        print(i, ":", words[i]);
        i = i + 1;
    }

    var grid = [[1, 2], [3, 4]];
    print("2d:", grid);
    grid[1][0] = 42;
    print("after edit:", grid);
    return 0;
}
