fn main() {
    println("Welcome to IO Lang Beta!");
    println("What's your name?");
    
    let name = read_line();
    println("Nice to meet you, " + name + "!");
    
    println("Let me show you some basic arithmetic...");
    let numbers = [1, 2, 3, 4, 5];
    let sum = 0;
    
    println("Numbers: " + to_string(numbers));
    
    for num in numbers {
        sum = sum + num;
    }
    
    println("Sum of numbers: " + to_string(sum));
    println("Average: " + to_string(sum / len(numbers)));
}
