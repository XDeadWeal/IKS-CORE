fn main() -> int {
    print("Guess the number (1..10)");
    let target = 7;
    var guess = 0;
    while (guess != target) {
        guess = int(input("your guess: "));
        if (guess < target) {
            print("too low");
        } else if (guess > target) {
            print("too high");
        } else {
            print("you win!");
        }
    }
    return 0;
}
