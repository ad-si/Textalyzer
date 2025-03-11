#!/usr/bin/env python3
"""
Example script that contains multi-line duplications.
This is to demonstrate the Textalyzer duplication detection.
"""

def function_one():
    # This block will be repeated in other functions
    print("This is the first line in a block")
    print("This is the second line in a block")
    print("This is the third line in a block")
    
    # This is unique to function_one
    print("This line only appears in function_one")
    
    # This block is repeated in all functions
    return "All functions return this exact string"

def function_two():
    # Some unique code to function_two
    print("Starting function_two")
    
    # This block will be repeated in other functions
    print("This is the first line in a block")
    print("This is the second line in a block")
    print("This is the third line in a block")
    
    # This block is repeated in all functions
    return "All functions return this exact string"

def function_three():
    # Unique section
    x = 10
    y = 20
    z = x + y
    print(f"The sum is {z}")
    
    # Another common block with slight variation
    print("This is the first line in a block")
    print("This is the second line in a block")
    print("This line is different in function_three")
    
    # This block is repeated in all functions
    return "All functions return this exact string"

# All of these functions are called from main
def main():
    function_one()
    function_two()
    function_three()

if __name__ == "__main__":
    main()
