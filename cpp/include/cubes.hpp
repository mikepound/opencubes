#pragma once
#ifndef OPENCUBES_CUBES_HPP
#define OPENCUBES_CUBES_HPP

#include "hashes.hpp"

Hashy gen(int n, int threads = 1, bool use_cache = false, bool write_cache = false);
#endif