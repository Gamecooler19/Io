fn main() {
    println("Hello, World!");
    
    let name = read_line();
    println("Hello, " + name + "!");
    
    let numbers = [1, 2, 3, 4, 5];
    let sum = 0;
    
    for num in numbers {
        sum = sum + num;
    }
    
    println("Sum: " + to_string(sum));
}
