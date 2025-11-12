package main

func add(a int, b int) int {
    return a + b
}

func multiply(x int, y int) int {
    return x * y
}

type Calculator struct{}

func (c Calculator) Divide(a int, b int) int {
    return a / b
}
