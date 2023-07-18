cmake_minimum_required(VERSION 3.14)
project(cubes CXX)

# default to release build because speed maters.
if(NOT CMAKE_BUILD_TYPE)
    set(CMAKE_BUILD_TYPE "Release")
endif()

include_directories("include")
include_directories("libraries")

macro(ConfigureTarget Target)
	# Enable C++17
	target_compile_features(${Target} PUBLIC cxx_std_17)
	target_compile_definitions(${Target} PUBLIC
	# Debug defines:
		$<$<CONFIG:Debug>:DEBUG>
	# Release defines:
		$<$<CONFIG:Release>:NDEBUG>
		$<$<CONFIG:RelWithDebInfo>:NDEBUG>
	)
	target_compile_options(${Target} PUBLIC
	# Flags used for all build types:
		-Wall -Wextra
	# Debug build flags:
		$<$<CONFIG:Debug>:-O0>
		$<$<CONFIG:Debug>:-Werror>
		$<$<CONFIG:Debug>:-Wno-unknown-pragmas>
	# Release build flags:
		$<$<CONFIG:Release>:-O3>
		$<$<CONFIG:Release>:-march=native>
		$<$<CONFIG:Release>:-Wno-unknown-pragmas>
	# Optimized with debug info (good for profiling the code)
		$<$<CONFIG:RelWithDebInfo>:-march=native>
		$<$<CONFIG:RelWithDebInfo>:-O3>
		$<$<CONFIG:RelWithDebInfo>:-g>
		$<$<CONFIG:RelWithDebInfo>:-fno-omit-frame-pointer>
	)
endmacro()

# Source files
add_library(CubeObjs OBJECT
	"src/cubes.cpp"
	"src/cache.cpp"
	"src/rotations.cpp"
)
ConfigureTarget(CubeObjs)

# Build main program
add_executable(${PROJECT_NAME} "program.cpp" $<TARGET_OBJECTS:CubeObjs>)
target_link_libraries(${PROJECT_NAME} pthread)
ConfigureTarget(${PROJECT_NAME})

# Optionally build tests
option(BUILD_TESTS OFF "Build test suite")
if(BUILD_TESTS)
	add_subdirectory(tests)
endif()
