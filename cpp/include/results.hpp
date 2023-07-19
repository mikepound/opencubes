#pragma once
#ifndef OPENCUBES_RESULTS_HPP
#define OPENCUBES_RESULTS_HPP
#include <cstdint>
#include <cstdio>

// from http://kevingong.com/Polyominoes/Enumeration.html
uint64_t results[] = {1, 1, 2, 8, 29, 166, 1023, 6922, 48311, 346543, 2522522, 18598427, 138462649, 1039496297, 7859514470, 59795121480};
static void checkResult(uint32_t n, uint64_t count) {
    if (sizeof(results) / sizeof(results[0]) > ((uint64_t)(n - 1)) && n > 1) {
        if (results[n - 1] != count) {
            std::printf("ERROR: result does not equal resultstable (%lu)!\n\r", results[n - 1]);
            std::exit(-1);
        }
    }
}
#endif
