cmake_minimum_required(VERSION 3.17)
project(pna)

set(CMAKE_CXX_STANDARD 20)

add_library(libpna libpna.h libpna.cpp)

add_executable(pna main.cpp)
