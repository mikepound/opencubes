#pragma once
#ifndef OPENCUBES_UTILS_HPP
#define OPENCUBES_UTILS_HPP

#include <cstdio>
#ifdef DEBUG
#define DEBUG_PRINTF(...) std::printf(__VA_ARGS__)
#else
#define DEBUG_PRINTF(...) \
    do {                  \
    } while (0)
#endif

#endif
