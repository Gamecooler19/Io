fn main() {
    println("Simple Calculator");
    println("Enter first number:");
    let a = parse_int(read_line());
    
    println("Enter second number:");
    let b = parse_int(read_line());
    
    println("Choose operation:");
    println("1. Add");
    println("2. Subtract");
    println("3. Multiply");
    println("4. Divide");
    
    let choice = parse_int(read_line());
    let result = 0;
    
    if choice == 1 {
        result = a + b;
        println("Sum = " + to_string(result));
    } else if choice == 2 {
        result = a - b;
        println("Difference = " + to_string(result));
    } else if choice == 3 {
        result = a * b;
        println("Product = " + to_string(result));
    } else if choice == 4 {
        if b != 0 {
            result = a / b;
            println("Quotient = " + to_string(result));
        } else {
            println("Error: Cannot divide by zero!");
        }
    } else {
        println("Invalid choice!");
    }
}