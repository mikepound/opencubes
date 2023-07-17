cmake_minimum_required(VERSION 3.14)
project(cubes_tests CXX)

# download google test
include(FetchContent)

FetchContent_Declare(
  googletest
  GIT_REPOSITORY https://github.com/google/googletest.git
  GIT_TAG        release-1.11.0
)
FetchContent_MakeAvailable(googletest)
add_library(GTest::GTest INTERFACE IMPORTED)

# include the test files
file(GLOB TESTS "src/*.cpp")

add_executable(${PROJECT_NAME} $<TARGET_OBJECTS:CubeObjs> ${TESTS})

target_link_libraries(GTest::GTest INTERFACE gtest_main)
target_link_libraries(${PROJECT_NAME} pthread GTest::GTest)
ConfigureTarget(${PROJECT_NAME})
