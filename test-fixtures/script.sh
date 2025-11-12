#!/bin/bash

add() {
    local a=$1
    local b=$2
    echo $((a + b))
}

multiply() {
    local x=$1
    local y=$2
    echo $((x * y))
}

divide() {
    local a=$1
    local b=$2
    echo $((a / b))
}
