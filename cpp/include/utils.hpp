#pragma once
#ifndef OPENCUBES_UTILS_HPP
#define OPENCUBES_UTILS_HPP

#include <cstdio>

// Debug print level: all prints enabled
// below DEBUG_LEVEL.
// DEBUG_LEVEL -> 0 all prints disabled.
// DEBUG_LEVEL -> 1 enable DEBUG_PRINTF() statements
// DEBUG_LEVEL -> 2 enable DEBUG1_PRINTF() statements and earlier
// DEBUG_LEVEL -> 3 all prints enabled.
#define DEBUG_LEVEL 1

#ifdef DEBUG

#if DEBUG_LEVEL >= 1
#define DEBUG_PRINTF(...) std::printf(__VA_ARGS__)
#endif

#if DEBUG_LEVEL >= 2
#define DEBUG1_PRINTF(...) std::printf(__VA_ARGS__)
#endif

#if DEBUG_LEVEL >= 3
#define DEBUG2_PRINTF(...) std::printf(__VA_ARGS__)
#endif

#endif

#ifndef DEBUG_PRINTF
#define DEBUG_PRINTF(...) do {} while (0)
#endif
#ifndef DEBUG1_PRINTF
#define DEBUG1_PRINTF(...) do {} while (0)
#endif
#ifndef DEBUG2_PRINTF
#define DEBUG2_PRINTF(...) do {} while (0)
#endif

#endif