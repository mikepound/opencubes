cmake_minimum_required(VERSION 3.14)
project(cubes_tests)

set(CMAKE_BUILD_TYPE "Debug")
set(CMAKE_CXX_FLAGS "-std=c++17")
set(CMAKE_CXX_FLAGS_DEBUG "-march=native -O -Wall -Werror -Wextra -D DEBUG")

# download google test
include(FetchContent)

FetchContent_Declare(
  googletest
  GIT_REPOSITORY https://github.com/google/googletest.git
  GIT_TAG        release-1.11.0
)
FetchContent_MakeAvailable(googletest)
add_library(GTest::GTest INTERFACE IMPORTED)

# include the source files
include_directories("../include")
file(GLOB SOURCES "../src/*.cpp")

# include the test files
include_directories("include")
file(GLOB TESTS "src/*.cpp")

add_executable(${PROJECT_NAME} ${SOURCES} ${TESTS})
target_link_libraries(GTest::GTest INTERFACE gtest_main)
target_link_libraries(${PROJECT_NAME} pthread GTest::GTest)
target_compile_definitions(${PROJECT_NAME} PUBLIC)