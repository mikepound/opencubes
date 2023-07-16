cmake_minimum_required(VERSION 3.0.2)
project(cubes)

set(CMAKE_BUILD_TYPE "Release")
set(CMAKE_CXX_FLAGS "-std=c++17")
set(CMAKE_CXX_FLAGS_RELEASE "-march=native -O3 -Wall -Wextra")
set(CMAKE_CXX_FLAGS_DEBUG "-march=native -O -Wall -Werror -Wextra -D DEBUG")

include_directories("include")
include_directories("libraries")
file(GLOB SOURCES "src/*.cpp")
add_executable(${PROJECT_NAME} ${SOURCES})
target_link_libraries(${PROJECT_NAME} pthread)
target_compile_definitions(${PROJECT_NAME} PUBLIC NDEBUG)