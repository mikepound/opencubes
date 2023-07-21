#pragma once
#ifndef OPENCUBES_CUBES_HPP
#define OPENCUBES_CUBES_HPP

#include "hashes.hpp"
#include "newCache.hpp"

FlatCache gen(int n, int threads = 1, bool use_cache = false, bool write_cache = false, bool split_cache = false, bool use_split_cache = false);
#endif